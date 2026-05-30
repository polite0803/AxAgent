const fs = require('fs');
const path = require('path');
const localesDir = path.join(__dirname, 'locales');

function fk(o,p){p=p||'';let r={};for(const k of Object.keys(o)){const f=p?p+'.'+k:k;const v=o[k];if(typeof v==='object'&&v!==null&&!Array.isArray(v))Object.assign(r,fk(v,f));else r[f]=v}return r}

const enFlat = fk(JSON.parse(fs.readFileSync(path.join(localesDir,'en-US.json'),'utf-8')));
const langs = ['ja','ko','fr','de','es','ru','ar','hi'];

const enOnlyByLang = {};
const allEnOnlyValues = {};

for (const lang of langs) {
  const flat = fk(JSON.parse(fs.readFileSync(path.join(localesDir,lang+'.json'),'utf-8')));
  const enOnly = [];
  for (const [k,v] of Object.entries(flat)) {
    if (typeof v !== 'string') continue;
    const ev = enFlat[k];
    if (ev === undefined) continue;
    if (v === ev && v.length > 2) {
      enOnly.push({ key: k, value: ev });
      if (!allEnOnlyValues[ev]) allEnOnlyValues[ev] = { count: 0, langs: new Set(), keys: [] };
      allEnOnlyValues[ev].count++;
      allEnOnlyValues[ev].langs.add(lang);
      allEnOnlyValues[ev].keys.push(k);
    }
  }
  enOnlyByLang[lang] = enOnly;
  console.log(`${lang}: ${enOnly.length} EN_ONLY strings`);
}

console.log('\n--- Unique EN_ONLY values appearing in ALL 8 languages ---');
const allLangs = Object.entries(allEnOnlyValues)
  .filter(([,v]) => v.langs.size === 8)
  .sort((a,b) => b[1].count - a[1].count);
console.log(`Total: ${allLangs.length} unique values`);
for (const [val, info] of allLangs) {
  console.log(`  "${val}" (appears ${info.count} times across languages)`);
}

console.log('\n--- All unique EN_ONLY values (sorted by frequency) ---');
const sorted = Object.entries(allEnOnlyValues).sort((a,b) => b[1].langs.size - a[1].langs.size || b[1].count - a[1].count);
for (const [val, info] of sorted) {
  console.log(`  "${val}" [${info.langs.size} langs, ${info.count} occurrences]`);
}
