import { RedisClient } from "bun";

const PROXY_URL = process.env.PROXY_URL || "redis://localhost:16379";
const NUM_CLIENTS = parseInt(process.env.NUM_CLIENTS || "10");
const OPS_PER_CLIENT = parseInt(process.env.OPS_PER_CLIENT || "1000");

interface BenchResult {
  clientId: number;
  ops: number;
  durationMs: number;
  opsPerSec: number;
}

async function runClient(clientId: number, opsCount: number): Promise<BenchResult> {
  const client = new RedisClient(PROXY_URL);
  const keyPrefix = `bench:client${clientId}`;

  const start = performance.now();

  for (let i = 0; i < opsCount; i++) {
    const key = `${keyPrefix}:${i % 100}`; // Reuse 100 keys per client

    // Mix of operations: 50% SET, 40% GET, 10% INCR
    const op = i % 10;
    if (op < 5) {
      await client.set(key, `value-${i}`);
    } else if (op < 9) {
      await client.get(key);
    } else {
      await client.incr(`${keyPrefix}:counter`);
    }
  }

  const durationMs = performance.now() - start;

  // Cleanup
  for (let i = 0; i < 100; i++) {
    await client.del(`${keyPrefix}:${i}`);
  }
  await client.del(`${keyPrefix}:counter`);

  client.close();

  return {
    clientId,
    ops: opsCount,
    durationMs,
    opsPerSec: (opsCount / durationMs) * 1000,
  };
}

async function runBenchmark() {
  console.log("=== Redis Proxy Benchmark ===\n");
  console.log(`Proxy URL: ${PROXY_URL}`);
  console.log(`Clients: ${NUM_CLIENTS}`);
  console.log(`Operations per client: ${OPS_PER_CLIENT}`);
  console.log(`Total operations: ${NUM_CLIENTS * OPS_PER_CLIENT}\n`);

  // Warmup
  console.log("Warming up...");
  const warmupClient = new RedisClient(PROXY_URL);
  for (let i = 0; i < 100; i++) {
    await warmupClient.set("warmup", "value");
    await warmupClient.get("warmup");
  }
  await warmupClient.del("warmup");
  warmupClient.close();
  console.log("Warmup complete.\n");

  // Run benchmark with parallel clients
  console.log(`Starting ${NUM_CLIENTS} parallel clients...`);
  const totalStart = performance.now();

  const results = await Promise.all(
    Array.from({ length: NUM_CLIENTS }, (_, i) => runClient(i, OPS_PER_CLIENT))
  );

  const totalDurationMs = performance.now() - totalStart;
  const totalOps = NUM_CLIENTS * OPS_PER_CLIENT;

  // Print results
  console.log("\n--- Per-Client Results ---");
  for (const result of results) {
    console.log(
      `  Client ${result.clientId}: ${result.ops} ops in ${result.durationMs.toFixed(1)}ms (${result.opsPerSec.toFixed(0)} ops/sec)`
    );
  }

  const avgOpsPerSec = results.reduce((sum, r) => sum + r.opsPerSec, 0) / results.length;
  const totalOpsPerSec = (totalOps / totalDurationMs) * 1000;

  console.log("\n--- Summary ---");
  console.log(`Total time: ${totalDurationMs.toFixed(1)}ms`);
  console.log(`Total operations: ${totalOps}`);
  console.log(`Throughput: ${totalOpsPerSec.toFixed(0)} ops/sec`);
  console.log(`Avg per-client: ${avgOpsPerSec.toFixed(0)} ops/sec`);
  console.log("");
}

// Also run a latency test
async function runLatencyTest() {
  console.log("=== Latency Test ===\n");

  const client = new RedisClient(PROXY_URL);
  const samples = 1000;
  const latencies: number[] = [];

  for (let i = 0; i < samples; i++) {
    const start = performance.now();
    await client.ping();
    latencies.push(performance.now() - start);
  }

  client.close();

  latencies.sort((a, b) => a - b);

  const avg = latencies.reduce((a, b) => a + b, 0) / latencies.length;
  const min = latencies[0];
  const max = latencies[latencies.length - 1];
  const p50 = latencies[Math.floor(latencies.length * 0.5)];
  const p95 = latencies[Math.floor(latencies.length * 0.95)];
  const p99 = latencies[Math.floor(latencies.length * 0.99)];

  console.log(`Samples: ${samples} PING commands`);
  console.log(`Min:  ${min.toFixed(3)}ms`);
  console.log(`Max:  ${max.toFixed(3)}ms`);
  console.log(`Avg:  ${avg.toFixed(3)}ms`);
  console.log(`P50:  ${p50.toFixed(3)}ms`);
  console.log(`P95:  ${p95.toFixed(3)}ms`);
  console.log(`P99:  ${p99.toFixed(3)}ms`);
  console.log("");
}

async function main() {
  try {
    await runLatencyTest();
    await runBenchmark();
    console.log("✓ Benchmark complete!");
  } catch (error) {
    console.error("Benchmark failed:", error);
    process.exit(1);
  }
}

main();
