const fs = require('fs');
const path = require('path');
const dir = 'metaclouds-backend-rust/k8s';
const imgRe = /^ghcr\.io\/[a-zA-Z0-9_.-]+\/[a-zA-Z0-9_.-]+\/[a-zA-Z0-9_.-]+(?::[a-zA-Z0-9_.-]+)?$/;
let errors = [];
for (const fn of fs.readdirSync(dir).filter(f => f.endsWith('.yaml') || f.endsWith('.yml'))) {
  const content = fs.readFileSync(path.join(dir, fn), 'utf8');
  for (const line of content.split('\n')) {
    if (!line.includes('image:')) continue;
    const m = line.match(/ghcr\.io\/\S+/);
    if (m && !imgRe.test(m[0].replace(/[,"']$/, ''))) errors.push(fn + ': non-hierarchical image ref: ' + m[0]);
  }
}
const kust = path.join(dir, '13-kustomization.yaml');
if (fs.existsSync(kust)) {
  const kt = fs.readFileSync(kust, 'utf8');
  const resLines = kt.split('\n').filter(l => /^\s*-\s+[0-9a-zA-Z\-_./]+\.yaml/.test(l));
  for (const l of resLines) {
    const res = l.trim().replace(/^-\s+/, '');
    if (!fs.existsSync(path.join(dir, res))) errors.push('kustomization references missing resource: ' + res);
  }
  const imgLines = kt.split('\n').filter(l => l.includes('name: ghcr.io'));
  for (const l of imgLines) {
    const name = l.split('name:')[1].trim();
    if (!imgRe.test(name)) errors.push('kustomization image not hierarchical: ' + name);
  }
}
if (errors.length) { console.log('ERRORS:'); errors.forEach(e => console.log('  - ' + e)); process.exit(1); }
else console.log('node checks PASSED (rules 2 and 3)');
