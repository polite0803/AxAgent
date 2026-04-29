
import fs from 'fs';

const de = JSON.parse(fs.readFileSync('src/i18n/locales/de.json', 'utf8'));
console.log('de.skills exists:', !!de.skills);
console.log('de.skills keys:', Object.keys(de.skills || {}));
console.log('de.skills.marketplace exists:', !!de.skills?.marketplace);
