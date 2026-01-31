import { RedisClient } from "bun";

const PROXY_URL = process.env.PROXY_URL || "redis://localhost:16379";

async function main() {
  console.log(`Connecting to proxy at ${PROXY_URL}...\n`);

  const client = new RedisClient(PROXY_URL);

  try {
    // Test PING
    console.log("Testing PING...");
    const pong = await client.ping();
    console.log(`  PING -> ${pong}`);

    // Test SET/GET
    console.log("\nTesting SET/GET...");
    await client.set("test:key1", "hello world");
    console.log('  SET test:key1 "hello world" -> OK');

    const value = await client.get("test:key1");
    console.log(`  GET test:key1 -> "${value}"`);

    // Test INCR/DECR
    console.log("\nTesting INCR/DECR...");
    await client.set("test:counter", "0");
    await client.incr("test:counter");
    await client.incr("test:counter");
    await client.incr("test:counter");
    const counter = await client.get("test:counter");
    console.log(`  INCR test:counter (3x) -> ${counter}`);

    await client.decr("test:counter");
    const decremented = await client.get("test:counter");
    console.log(`  DECR test:counter -> ${decremented}`);

    // Test multiple SET/GET operations
    console.log("\nTesting batch operations...");
    for (let i = 0; i < 10; i++) {
      await client.set(`test:batch:${i}`, `value-${i}`);
    }
    console.log("  SET test:batch:0..9 -> OK");

    for (let i = 0; i < 10; i++) {
      await client.get(`test:batch:${i}`);
    }
    console.log("  GET test:batch:0..9 -> OK");

    // Test EXISTS/DEL
    console.log("\nTesting EXISTS/DEL...");
    const exists = await client.exists("test:key1");
    console.log(`  EXISTS test:key1 -> ${exists}`);

    await client.del("test:key1");
    console.log("  DEL test:key1 -> OK");

    const existsAfter = await client.exists("test:key1");
    console.log(`  EXISTS test:key1 -> ${existsAfter}`);

    // Test EXPIRE/TTL
    console.log("\nTesting EXPIRE/TTL...");
    await client.set("test:expiring", "temporary");
    await client.expire("test:expiring", 60);
    const ttl = await client.ttl("test:expiring");
    console.log(`  EXPIRE test:expiring 60 -> TTL is ${ttl}s`);

    // Test hash operations
    console.log("\nTesting hash operations...");
    await client.send("HSET", ["test:user", "name", "Alice", "email", "alice@example.com"]);
    console.log("  HSET test:user name Alice email alice@example.com -> OK");

    const name = await client.hget("test:user", "name");
    console.log(`  HGET test:user name -> "${name}"`);

    // Cleanup
    console.log("\nCleaning up...");
    await client.del("test:counter");
    await client.del("test:expiring");
    await client.del("test:user");
    for (let i = 0; i < 10; i++) {
      await client.del(`test:batch:${i}`);
    }
    console.log("  Cleanup complete");

    console.log("\n✓ All tests passed!");
  } catch (error) {
    console.error("\n✗ Test failed:", error);
    process.exit(1);
  } finally {
    client.close();
  }
}

main();
