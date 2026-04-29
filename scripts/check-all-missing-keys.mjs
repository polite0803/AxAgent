
import fs from 'fs';

const en = JSON.parse(fs.readFileSync('src/i18n/locales/en-US.json', 'utf8'));

function findMissingKeys(enObj, langObj, prefix = '') {
  const results = [];
  for (const k in enObj) {
    const p = prefix ? prefix + '.' + k : k;
    if (typeof enObj[k] === 'object' && enObj[k] !== null && !Array.isArray(enObj[k])) {
      if (langObj[k] && typeof langObj[k] === 'object') {
        results.push(...findMissingKeys(enObj[k], langObj[k], p));
      } else {
        results.push({ path: p, type: 'object', value: enObj[k] });
      }
    } else if (!(k in langObj)) {
      results.push({ path: p, value: enObj[k], type: 'string' });
    }
  }
  return results;
}

const langs = [
  'ar.json', 'de.json', 'es.json', 'fr.json',
  'hi.json', 'ja.json', 'ko.json', 'ru.json',
  'zh-TW.json'
];

for (const file of langs) {
  const langPath = 'src/i18n/locales/' + file;
  const lang = JSON.parse(fs.readFileSync(langPath, 'utf8'));
  const missing = findMissingKeys(en, lang);

  if (missing.length > 0) {
    console.log(`\n=== ${file} (${missing.length} missing keys) ===`);
    missing.forEach(m => {
      console.log(`  ${m.path} = "${m.type === 'string' ? m.value : '[object]'}"`);
    });
  } else {
    console.log(`\n=== ${file} - No missing keys ===`);
  }
}
