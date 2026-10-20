
const fs=require('fs');
const buf=fs.readFileSync('D:/ai hook/deepseekharness/resources/app.asar');
console.log('len', buf.length);
console.log('u32[0..5]', [0,4,8,12,16,20].map(o=>buf.readUInt32LE(o)));
console.log('head100', JSON.stringify(buf.slice(0,100).toString('latin1')));
const jsonSize=buf.readUInt32LE(8);
console.log('jsonSize', jsonSize);
const s=buf.slice(12, 12+jsonSize).toString('utf8');
console.log('parses?', (()=>{try{JSON.parse(s);return true}catch(e){return String(e).slice(0,120)}})());
console.log('tail of json', JSON.stringify(s.slice(-60)));
