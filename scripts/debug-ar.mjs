
import fs from 'fs';

const ar = JSON.parse(fs.readFileSync('src/i18n/locales/ar.json', 'utf8'));
console.log('ar.settings exists:', !!ar.settings);
console.log('ar.settings.theme exists:', !!ar.settings?.theme);
console.log('ar.settings keys:', Object.keys(ar.settings || {}));
console.log('ar.style exists:', !!ar.style);
console.log('ar.style.dimensions exists:', !!ar.style?.dimensions);
