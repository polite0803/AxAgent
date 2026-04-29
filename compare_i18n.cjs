const fs = require('fs');
const path = require('path');

const enPath = path.join(__dirname, 'src', 'i18n', 'locales', 'en-US.json');
const zhPath = path.join(__dirname, 'src', 'i18n', 'locales', 'zh-CN.json');

const en = JSON.parse(fs.readFileSync(enPath, 'utf8'));
const zh = JSON.parse(fs.readFileSync(zhPath, 'utf8'));

function getKeysWithValues(obj, prefix = '') {
    let keys = [];
    for (const key in obj) {
        const fullKey = prefix ? `${prefix}.${key}` : key;
        if (typeof obj[key] === 'object' && obj[key] !== null) {
            keys = keys.concat(getKeysWithValues(obj[key], fullKey));
        } else {
            keys.push({ key: fullKey, value: obj[key] });
        }
    }
    return keys;
}

const enKeys = getKeysWithValues(en);
const zhKeys = getKeysWithValues(zh);

const zhMap = new Map(zhKeys.map(kv => [kv.key, kv.value]));

const missingOrEmpty = enKeys.filter(kv => {
    const zhValue = zhMap.get(kv.key);
    return zhValue === undefined || zhValue === '' || zhValue === null;
});

console.log(`Missing or empty keys: ${missingOrEmpty.length}`);
missingOrEmpty.forEach(kv => {
    const zhValue = zhMap.get(kv.key);
    console.log(`${kv.key}: en="${kv.value}" zh="${zhValue}"`);
});
