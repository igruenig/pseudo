import { spawn } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const args = process.argv.slice(2);
const env = { ...process.env };

if (process.platform === "darwin") {
  env.MACOSX_DEPLOYMENT_TARGET = env.MACOSX_DEPLOYMENT_TARGET ?? "11.0";
  env.CXXFLAGS = appendFlag(env.CXXFLAGS, "-D_LIBCPP_DISABLE_AVAILABILITY");
  env.CMAKE_CXX_FLAGS = appendFlag(env.CMAKE_CXX_FLAGS, "-D_LIBCPP_DISABLE_AVAILABILITY");
}

const child = spawn("tauri", args, {
  cwd: join(root, "src-tauri", "app"),
  env,
  shell: process.platform === "win32",
  stdio: "inherit"
});

child.on("exit", (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
    return;
  }
  process.exit(code ?? 1);
});

function appendFlag(value, flag) {
  if (!value) return flag;
  if (value.split(/\s+/).includes(flag)) return value;
  return `${value} ${flag}`;
}
