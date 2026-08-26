import { spawnSync } from "node:child_process";

const cargo = process.platform === "win32" ? "cargo.exe" : "cargo";
const commands = [
  ["Format", ["fmt", "--manifest-path", "src-tauri/Cargo.toml", "--", "--check"]],
  [
    "Clippy",
    [
      "clippy",
      "--manifest-path",
      "src-tauri/Cargo.toml",
      "--all-targets",
      "--all-features",
      "--",
      "-D",
      "warnings",
    ],
  ],
  [
    "Tests",
    [
      "test",
      "--manifest-path",
      "src-tauri/Cargo.toml",
      "--no-default-features",
      "--lib",
    ],
  ],
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
