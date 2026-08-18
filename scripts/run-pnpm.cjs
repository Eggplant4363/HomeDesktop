// 项目内 pnpm 启动器：自动定位 pnpm.cjs 并透传参数（用于 tauri beforeDev/beforeBuildCommand）
// 本机（Node 24 + npm 11 zlib 兼容问题）走 corepack pnpm；CI 上回退到 PATH 中的 pnpm。
const { spawn } = require("node:child_process");
const path = require("node:path");
const fs = require("node:fs");

const projectRoot = path.resolve(__dirname, "..");
const args = process.argv.slice(2);

const candidates = [
  path.join(projectRoot, ".corepack", "v1", "pnpm", "11.22.0", "bin", "pnpm.cjs"),
  ...(process.env.COREPACK_HOME
    ? [path.join(process.env.COREPACK_HOME, "v1", "pnpm", "11.22.0", "bin", "pnpm.cjs")]
    : []),
];

const pnpm = candidates.find((p) => fs.existsSync(p));

// stdio inherit：避免子进程管道捕获在受限沙箱下触发 EPERM
const child = pnpm
  ? spawn(process.execPath, [pnpm, ...args], { cwd: projectRoot, stdio: "inherit" })
  : spawn("pnpm", args, { cwd: projectRoot, stdio: "inherit", shell: true });

child.on("exit", (code) => process.exit(code ?? 0));
child.on("error", (e) => {
  console.error("[homedesktop] spawn pnpm failed:", e);
  process.exit(1);
});
