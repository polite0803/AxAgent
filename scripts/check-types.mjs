
import fs from 'fs';

const ar = JSON.parse(fs.readFileSync('src/i18n/locales/ar.json', 'utf8'));
console.log('settings.theme:', ar.settings.theme);
console.log('type:', typeof ar.settings.theme);
console.log('is object:', typeof ar.settings.theme === 'object' && ar.settings.theme !== null && !Array.isArray(ar.settings.theme));
console.log('keys:', Object.keys(ar.settings.theme || {}));
console.log('settings.shortcuts:', ar.settings.shortcuts);
console.log('type:', typeof ar.settings.shortcuts);
console.log('style.dimensions:', ar.style.dimensions);
console.log('type:', typeof ar.style.dimensions);
