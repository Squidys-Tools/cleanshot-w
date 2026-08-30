import { spawnSync } from "node:child_process";

const cargo = process.platform === "win32" ? "cargo.exe" : "cargo";

// windows-gnu test harness exes lack the comctl32 v6 manifest tauri embeds in
// the real binary, so they abort at load (STATUS_ENTRYPOINT_NOT_FOUND on
// TaskDialogIndirect). Rust tests stay on CI's msvc runner; locally we run
// fmt + clippy only.
const isWindowsGnu =
  process.platform === "win32" &&
  (process.env.RUSTUP_TOOLCHAIN ?? "").endsWith("-windows-gnu");

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

const applicable = isWindowsGnu
  ? commands.filter(([name]) => name !== "Tests")
  : commands;

for (const [name, args] of applicable) {
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
