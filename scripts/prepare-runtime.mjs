import { mkdir, readFile, writeFile, copyFile, cp, access, chmod, rm } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const version = '24.21.0';
const platform = process.platform === 'win32' ? 'win' : process.platform;
const archives = {
  'darwin-arm64': ['tar.gz', 'bed7eea5325e1108f32ce5228ddd6a5f0f08a499ee42aa7442aea583702f6057'],
  'darwin-x64': ['tar.gz', '1462cb3b3046b815cf8ea436d3da450ec1a9f11dac7e5a46b0ada5305d7e8097'],
  'linux-x64': ['tar.gz', '6e1db87ef58b8819e5d5402eff1536491b18edd8eb7bee5ef7897876e88dc5ff'],
  'linux-arm64': ['tar.gz', '724282c3b43aec998aa9527380465b45d229e021b58035f5f4f63095eabfe5d5'],
  'win-x64': ['zip', '158f7685b44de51f6c0df1d153526cbcd3e1bc739a8dfc607721cef75de9e541'],
};
const target = `${platform}-${process.arch}`;
if (!archives[target]) throw new Error(`Node runtime target not configured: ${target}`);
const [extension, checksum] = archives[target];
const name = `node-v${version}-${target}`;
const cache = path.join(root, '.build', 'runtime');
const resources = path.join(root, 'src-tauri', 'runtime-resources');
await mkdir(cache, { recursive: true });
await mkdir(path.join(resources, 'bin'), { recursive: true });
const archive = path.join(cache, `${name}.${extension}`);
let data;
try { data = await readFile(archive); } catch {
  console.log(`Downloading bundled Node ${version} (${target})`);
  const response = await fetch(`https://nodejs.org/dist/v${version}/${name}.${extension}`);
  if (!response.ok) throw new Error(`Node download: HTTP ${response.status}`);
  data = Buffer.from(await response.arrayBuffer());
  await writeFile(archive, data);
}
if (createHash('sha256').update(data).digest('hex') !== checksum) throw new Error('Node archive checksum mismatch');
if (extension === 'zip') execFileSync('tar', ['-xf', archive, '-C', cache]);
else execFileSync('tar', ['-xzf', archive, '-C', cache]);
const binary = platform === 'win' ? 'node.exe' : 'node';
await copyFile(path.join(cache, name, ...(platform === 'win' ? [] : ['bin']), binary), path.join(resources, 'bin', binary));
await chmod(path.join(resources, 'bin', binary), 0o755);
await mkdir(path.join(resources, 'licenses'), { recursive: true });
await copyFile(path.join(cache, name, 'LICENSE'), path.join(resources, 'licenses', 'Node-LICENSE'));
const source = path.join(root, 'runtime', 'pi');
await access(path.join(source, 'node_modules', '@earendil-works', 'pi-coding-agent'));
await rm(path.join(resources,'pi'), {recursive:true,force:true});
await cp(source, path.join(resources, 'pi'), { recursive: true, verbatimSymlinks: true, filter: (p) => !p.includes(`${path.sep}test${path.sep}`) });
// Tauri's resource walker omits pnpm directory symlinks. Preserve the complete
// dependency graph in an archive and let Rust unpack it into an owned cache.
const piArchive=path.join(resources,'pi.tar.gz');
execFileSync('tar',['-czf',piArchive,'--exclude=./test','-C',path.join(resources,'pi'),'.'],{env:{...process.env,COPYFILE_DISABLE:'1'}});
const piArchiveSha256=createHash('sha256').update(await readFile(piArchive)).digest('hex');
await writeFile(path.join(resources, 'runtime-manifest.json'), JSON.stringify({ node: version, pi: '0.85.0', target, piArchiveSha256 }, null, 2));
console.log(`Bundled runtime prepared: ${target}`);
