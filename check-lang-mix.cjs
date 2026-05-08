const fs = require('fs');
const path = require('path');

const localeDir = 'd:/OneManager/AxAgent/src/i18n/locales';

const enUS = JSON.parse(fs.readFileSync(path.join(localeDir, 'en-US.json'), 'utf8'));
const ja = JSON.parse(fs.readFileSync(path.join(localeDir, 'ja.json'), 'utf8'));
const zhCN = JSON.parse(fs.readFileSync(path.join(localeDir, 'zh-CN.json'), 'utf8'));
const zhTW = JSON.parse(fs.readFileSync(path.join(localeDir, 'zh-TW.json'), 'utf8'));
const ko = JSON.parse(fs.readFileSync(path.join(localeDir, 'ko.json'), 'utf8'));
const ru = JSON.parse(fs.readFileSync(path.join(localeDir, 'ru.json'), 'utf8'));

function findNonNativeValues(obj, pattern, prefix = '') {
  const results = [];
  for (const [k, v] of Object.entries(obj)) {
    const fullKey = prefix ? prefix + '.' + k : k;
    if (typeof v === 'string' && pattern.test(v)) {
      results.push({ key: fullKey, value: v });
    } else if (v && typeof v === 'object' && !Array.isArray(v)) {
      results.push(...findNonNativeValues(v, pattern, fullKey));
    }
  }
  return results;
}

// en-US: Chinese characters (should not have any)
const cjkPattern = /[\u4e00-\u9fff\u3040-\u309f\u30a0-\u30ff]/;

console.log('=== en-US.json 中不应有的CJK字符 ===');
const chineseInEn = findNonNativeValues(enUS, cjkPattern);
if (chineseInEn.length > 0) {
  chineseInEn.slice(0, 30).forEach(s => {
    const shortVal = s.value.length > 50 ? s.value.substring(0, 50) + '...' : s.value;
    console.log('  ' + s.key + ' = "' + shortVal + '"');
  });
} else {
  console.log('  无');
}

console.log('\n=== ja.json 是否大部分是中文(未翻译)? ===');
const chineseInJa = findNonNativeValues(ja, /[\u4e00-\u9fff]/);
const japaneseInJa = findNonNativeValues(ja, /[\u3040-\u309f\u30a0-\u30ff]/);
console.log('  含中文字符的键: ' + chineseInJa.length);
console.log('  含日文字符的键: ' + japaneseInJa.length);
console.log('  总计字符串数: ' + countStrings(ja));

if (chineseInJa.length > 10) {
  console.log('  中文示例(前10):');
  chineseInJa.slice(0, 10).forEach(s => {
    const shortVal = s.value.length > 50 ? s.value.substring(0, 50) + '...' : s.value;
    console.log('    ' + s.key + ' = "' + shortVal + '"');
  });
}

console.log('\n=== zh-TW/ko/ru 中的日文混入 ===');
const jaPattern = /[\u3040-\u309f\u30a0-\u30ff]/;
for (const [file, data] of [['zh-TW.json', zhTW], ['ko.json', ko], ['ru.json', ru]]) {
  const jaInFile = findNonNativeValues(data, jaPattern);
  if (jaInFile.length > 0) {
    console.log('  ' + file + ': ' + jaInFile.length + ' 条日文混入');
    jaInFile.slice(0, 5).forEach(s => {
      console.log('    ' + s.key + ' = "' + s.value + '"');
    });
  }
}

console.log('\n=== zh-CN 中的非简体字符 ===');
const nonSimplifiedInZhCN = findNonNativeValues(zhCN, /[\u3040-\u309f\u30a0-\u30ff\uac00-\ud77f]/);
if (nonSimplifiedInZhCN.length > 0) {
  nonSimplifiedInZhCN.slice(0, 10).forEach(s => {
    console.log('  ' + s.key + ' = "' + s.value + '"');
  });
} else {
  console.log('  无');
}

function countStrings(obj) {
  let count = 0;
  for (const [k, v] of Object.entries(obj)) {
    if (typeof v === 'string') count++;
    else if (v && typeof v === 'object' && !Array.isArray(v)) count += countStrings(v);
  }
  return count;
}
