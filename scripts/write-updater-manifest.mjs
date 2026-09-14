// 生成 Tauri 自动更新清单（latest.json / latest-beta.json）。
//
// 输入来自 release.yml 的打包产物：target/<triple>/release/bundle/macos/Fleqi.app.tar.gz(.sig)。
// 清单里的 url 必须指向本 tag 的 Release 资产（不能用 "latest" 下载地址，
// 否则 beta 与 stable 会互相串包）。
//
// 用法：
//   node scripts/write-updater-manifest.mjs \
//     --bundle-dir src-tauri/target/aarch64-apple-darwin/release/bundle/macos \
//     --tag v0.0.1-beta.1 --arch aarch64 --channel beta
import { readFileSync, writeFileSync, existsSync, readdirSync } from 'node:fs';
import path from 'node:path';

function parseArgs(argv) {
  const args = {};
  for (let index = 0; index < argv.length; index += 1) {
    if (!argv[index].startsWith('--')) continue;
    args[argv[index].slice(2)] = argv[index + 1];
    index += 1;
  }
  return args;
}

const args = parseArgs(process.argv.slice(2));
const repository = args.repo ?? process.env.GITHUB_REPOSITORY ?? 'Open-Less/fleqi';
const tag = args.tag ?? process.env.GITHUB_REF_NAME;
const arch = args.arch ?? 'aarch64';
const channel = (args.channel ?? 'stable').trim();
const version = (args.version ?? tag.replace(/^v/, '')).trim();
const bundleDir = path.resolve(args['bundle-dir'] ?? 'src-tauri/target/aarch64-apple-darwin/release/bundle/macos');

if (!tag) {
  process.stderr.write('缺少 --tag（或 GITHUB_REF_NAME）\n');
  process.exit(1);
}
if (!existsSync(bundleDir)) {
  process.stderr.write(`打包目录不存在：${bundleDir}\n`);
  process.exit(1);
}

const files = readdirSync(bundleDir);
const archive = files.find((name) => name.endsWith('.app.tar.gz'));
if (!archive) {
  process.stderr.write(`未找到 .app.tar.gz（目录内：${files.join('、') || '空'}）。请确认打包时开启了 createUpdaterArtifacts。\n`);
  process.exit(1);
}
const signatureFile = `${archive}.sig`;
if (!files.includes(signatureFile)) {
  process.stderr.write(`未找到签名文件 ${signatureFile}。缺少 TAURI_SIGNING_PRIVATE_KEY 时不会生成更新包签名。\n`);
  process.exit(1);
}

const assetName = `Fleqi_${arch}.app.tar.gz`;
if (archive !== assetName) {
  writeFileSync(path.join(bundleDir, assetName), readFileSync(path.join(bundleDir, archive)));
  writeFileSync(path.join(bundleDir, `${assetName}.sig`), readFileSync(path.join(bundleDir, signatureFile)));
}

const manifest = {
  version,
  notes: args.notes ?? '',
  pub_date: new Date().toISOString(),
  platforms: {
    [`darwin-${arch}`]: {
      signature: readFileSync(path.join(bundleDir, `${assetName}.sig`), 'utf8').trim(),
      url: `https://github.com/${repository}/releases/download/${tag}/${assetName}`,
    },
  },
};
const manifestName = channel === 'beta' ? 'latest-beta.json' : 'latest.json';
const manifestPath = path.join(path.resolve(args['out-dir'] ?? bundleDir), manifestName);
writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
process.stdout.write(`${manifestPath}\n`);
