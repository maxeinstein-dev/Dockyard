// `pnpm check`: the CI's `frontend` and `rust` jobs on this machine, in the
// CI's order, stopping at the first failure. Run by the pre-push hook
// (.githooks/pre-push) so a red CI round, six minutes plus a branch update,
// is caught here in two or three. The e2e jobs stay in the CI: they open a
// second dock on the screen.
import { spawnSync } from "node:child_process";

const STEPS = [
  ["Lint", "pnpm lint"],
  ["Format check", "pnpm format:check"],
  ["Typecheck", "pnpm typecheck"],
  ["Test", "pnpm test"],
  // `tauri::generate_context!` requires `frontendDist` (../dist) at compile time.
  ["Build frontend", "pnpm build"],
  ["cargo fmt", "cargo fmt --check", "src-tauri"],
  ["Release build has no WebDriver", null, "src-tauri"],
  [
    "cargo clippy",
    "cargo clippy --all-targets --locked -- -D warnings",
    "src-tauri",
  ],
  ["cargo test", "cargo test --locked", "src-tauri"],
];

// Spec 0003, Verificação 8: the WebDriver server comes only with the `e2e` feature.
function noWebDriver(cwd) {
  const tree = spawnSync("cargo", ["tree", "--locked", "-e", "normal"], {
    cwd,
    encoding: "utf8",
    shell: true,
  });
  if (tree.status !== 0) {
    process.stderr.write(tree.stderr);
    return false;
  }
  const hits = tree.stdout
    .split("\n")
    .filter((line) => /wdio|webdriver/i.test(line));
  if (hits.length > 0) {
    console.error(
      "The dependency tree without the e2e feature has the WebDriver plugin:",
    );
    console.error(hits.join("\n"));
    return false;
  }
  return true;
}

const started = Date.now();
for (const [name, command, cwd] of STEPS) {
  const at = Date.now();
  console.log(`\n▶ ${name}`);
  const ok = command
    ? spawnSync(command, { cwd, stdio: "inherit", shell: true }).status === 0
    : noWebDriver(cwd);
  const seconds = Math.round((Date.now() - at) / 1000);
  if (!ok) {
    console.error(`\n✗ ${name} falhou (${seconds} s). Corrija antes do push.`);
    console.error("Para enviar mesmo assim: git push --no-verify");
    process.exit(1);
  }
  console.log(`✓ ${name} (${seconds} s)`);
}
console.log(
  `\n✓ pnpm check passou em ${Math.round((Date.now() - started) / 1000)} s`,
);
