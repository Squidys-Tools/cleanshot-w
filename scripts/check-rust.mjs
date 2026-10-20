import { spawnSync } from "node:child_process";

const cargo = process.platform === "win32" ? "cargo.exe" : "cargo";

const commands = [
  ["Format", ["fmt", "--manifest-path", "app/Cargo.toml", "--", "--check"]],
  [
    "Clippy",
    [
      "clippy",
      "--manifest-path",
      "app/Cargo.toml",
      "--all-targets",
      "--",
      "-D",
      "warnings",
    ],
  ],
  ["Tests", ["test", "--manifest-path", "app/Cargo.toml"]],
  ["Check", ["check", "--manifest-path", "app/Cargo.toml"]],
];

for (const [name, args] of commands) {
  console.log(`\n== Rust ${name} ==`);
  const result = spawnSync(cargo, args, { stdio: "inherit" });
  if (result.error) {
    console.error(`Could not start cargo for ${name}: ${result.error.message}`);
    process.exit(1);
  }
  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}
