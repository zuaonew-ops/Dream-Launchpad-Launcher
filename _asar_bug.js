
const fs=require('fs');
const buf=fs.readFileSync('D:/ai hook/deepseekharness/resources/app.asar');
const jsonSize = buf.readUInt32LE(8) - 4;
const header = JSON.parse(buf.slice(16, 16+jsonSize).toString('utf8'));
const files=[]; (function walk(node, prefix){ for (const [n,v] of Object.entries(node.files||{})){ const full=prefix?prefix+'/'+n:n; if (v.files) walk(v, full); else files.push(full);} })(header,'');
console.log(files.filter(f=>/bug.?reporter/i.test(f)).slice(0,40).join('\n') || 'none in asar');
