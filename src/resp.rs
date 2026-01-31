/// RESP (Redis Serialization Protocol) parser.
///
/// This module handles parsing of the Redis protocol to extract command names
/// from the client->server data stream.

/// Parse RESP protocol to extract command names from the buffer.
/// Returns the commands found and how many bytes were consumed.
pub fn parse_commands(buf: &[u8]) -> (Vec<String>, usize) {
    let mut commands = Vec::new();
    let mut pos = 0;

    while pos < buf.len() {
        // Commands are RESP arrays starting with '*'
        if buf[pos] != b'*' {
            // Inline command (space-separated) - find the command name
            if let Some(cmd) = parse_inline_command(&buf[pos..]) {
                commands.push(cmd.0);
                pos += cmd.1;
                continue;
            }
            break;
        }

        // Parse array: *<count>\r\n
        let Some((array_len, consumed)) = parse_integer(&buf[pos + 1..]) else {
            break; // Incomplete
        };
        pos += 1 + consumed;

        if array_len <= 0 {
            continue;
        }

        // First element is the command name (bulk string)
        if pos >= buf.len() || buf[pos] != b'$' {
            break;
        }

        let Some((str_len, consumed)) = parse_integer(&buf[pos + 1..]) else {
            break;
        };
        pos += 1 + consumed;

        if str_len < 0 {
            continue; // Null bulk string
        }

        let str_len = str_len as usize;
        if pos + str_len + 2 > buf.len() {
            break; // Incomplete
        }

        let command = String::from_utf8_lossy(&buf[pos..pos + str_len]).to_string();
        commands.push(command);
        pos += str_len + 2; // +2 for \r\n

        // Skip remaining array elements
        for _ in 1..array_len {
            if pos >= buf.len() {
                break;
            }

            match buf[pos] {
                b'$' => {
                    // Bulk string
                    let Some((len, consumed)) = parse_integer(&buf[pos + 1..]) else {
                        return (commands, 0); // Incomplete, but we got the command
                    };
                    pos += 1 + consumed;

                    if len >= 0 {
                        let len = len as usize;
                        if pos + len + 2 > buf.len() {
                            return (commands, 0);
                        }
                        pos += len + 2;
                    }
                }
                b'+' | b'-' | b':' => {
                    // Simple string, error, or integer - find \r\n
                    if let Some(end) = find_crlf(&buf[pos + 1..]) {
                        pos += 1 + end + 2;
                    } else {
                        return (commands, 0);
                    }
                }
                _ => break,
            }
        }
    }

    (commands, pos)
}

/// Parse an inline command (space-separated, ending with \r\n).
fn parse_inline_command(buf: &[u8]) -> Option<(String, usize)> {
    let crlf_pos = find_crlf(buf)?;
    let line = &buf[..crlf_pos];

    // First word is the command
    let cmd_end = line
        .iter()
        .position(|&b| b == b' ')
        .unwrap_or(line.len());

    if cmd_end == 0 {
        return None;
    }

    let command = String::from_utf8_lossy(&line[..cmd_end]).to_string();
    Some((command, crlf_pos + 2))
}

/// Parse a RESP integer (until \r\n), returns value and bytes consumed including \r\n.
fn parse_integer(buf: &[u8]) -> Option<(i64, usize)> {
    let crlf_pos = find_crlf(buf)?;
    let num_str = std::str::from_utf8(&buf[..crlf_pos]).ok()?;
    let num = num_str.parse().ok()?;
    Some((num, crlf_pos + 2))
}

/// Find position of \r\n in buffer.
fn find_crlf(buf: &[u8]) -> Option<usize> {
    buf.windows(2).position(|w| w == b"\r\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_command() {
        let buf = b"*1\r\n$4\r\nPING\r\n";
        let (commands, _) = parse_commands(buf);
        assert_eq!(commands, vec!["PING"]);
    }

    #[test]
    fn test_parse_command_with_args() {
        let buf = b"*3\r\n$3\r\nSET\r\n$3\r\nkey\r\n$5\r\nvalue\r\n";
        let (commands, _) = parse_commands(buf);
        assert_eq!(commands, vec!["SET"]);
    }

    #[test]
    fn test_parse_inline_command() {
        let buf = b"PING\r\n";
        let (commands, _) = parse_commands(buf);
        assert_eq!(commands, vec!["PING"]);
    }

    #[test]
    fn test_parse_multiple_commands() {
        let buf = b"*1\r\n$4\r\nPING\r\n*1\r\n$4\r\nPING\r\n";
        let (commands, _) = parse_commands(buf);
        assert_eq!(commands, vec!["PING", "PING"]);
    }
}
