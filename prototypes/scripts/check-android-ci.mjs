// Both checks share the emulator lifecycle owned by check-android-environment.sh.
for (const args of [["check-automerge.mjs", "--android"], ["check-android.mjs"]]) {
  const result = Bun.spawnSync([process.execPath, `${import.meta.dir}/${args[0]}`, ...args.slice(1)], {
    env: process.env, stdin: "ignore", stdout: "inherit", stderr: "inherit",
  });
  if (result.exitCode || result.signalCode) process.exit(result.exitCode || 1);
}
