// RustyPipe's QuickJS needs generated Android bindings. Use NDK headers for
// bindgen too, rather than accidentally compiling bindings against host libc.
import { readdirSync, existsSync } from 'node:fs';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
const ndk = process.env.NDK_HOME;
if (!ndk) throw new Error('Set NDK_HOME to Android NDK 27.2.12479018 (see docs/platforms/android.md).');
const host = { linux: 'linux-x86_64', darwin: 'darwin-x86_64', win32: 'windows-x86_64' }[process.platform];
if (!host) throw new Error(`Unsupported build host: ${process.platform}`);
const toolchain = join(ndk, 'toolchains', 'llvm', 'prebuilt', host);
const resources = join(toolchain, 'lib', 'clang');
const versions = readdirSync(resources).sort((a, b) => b.localeCompare(a, undefined, { numeric: true }));
if (!versions[0]) throw new Error('NDK Clang resource directory is missing');
const sysroot = join(toolchain, 'sysroot');
if (!existsSync(sysroot)) throw new Error('NDK sysroot is missing');
const args = process.argv.slice(2);
const result = spawnSync(process.execPath, ['node_modules/@tauri-apps/cli/tauri.js', 'android', ...args], {
  stdio: 'inherit',
  env: { ...process.env, BINDGEN_EXTRA_CLANG_ARGS: `--sysroot="${sysroot}" -resource-dir="${join(resources, versions[0])}" ${process.env.BINDGEN_EXTRA_CLANG_ARGS || ''}` },
});
if (result.error) throw result.error;
process.exit(result.status ?? 1);
