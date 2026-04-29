
import fs from 'fs';

const en = JSON.parse(fs.readFileSync('src/i18n/locales/en-US.json', 'utf8'));
const ar = JSON.parse(fs.readFileSync('src/i18n/locales/ar.json', 'utf8'));

function findMissingKeys(enObj, langObj, prefix = '') {
  const results = [];
  console.log(`Checking prefix: ${prefix}`);
  for (const k in enObj) {
    const p = prefix ? prefix + '.' + k : k;
    console.log(`  Key: ${k} (${p})`);
    if (typeof enObj[k] === 'object' && enObj[k] !== null && !Array.isArray(enObj[k])) {
      if (langObj[k] && typeof langObj[k] === 'object') {
        console.log(`    Recursing into ${k}`);
        results.push(...findMissingKeys(enObj[k], langObj[k], p));
      } else {
        console.log(`    Missing object: ${p}`);
        results.push({ path: p, type: 'object', value: enObj[k] });
      }
    } else if (!(k in langObj)) {
      console.log(`    Missing string: ${p}`);
      results.push({ path: p, value: enObj[k], type: 'string' });
    } else {
      console.log(`    OK: ${p}`);
    }
  }
  return results;
}

const missing = findMissingKeys(en, ar);
console.log('\nMissing keys:', missing);
