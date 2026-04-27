import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

// 获取当前目录路径
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// 语言文件目录
const localesDir = path.join(__dirname, 'src', 'i18n', 'locales');

// 读取基准文件（en-US.json）
const enUSPath = path.join(localesDir, 'en-US.json');
const enUS = JSON.parse(fs.readFileSync(enUSPath, 'utf8'));

// 读取所有语言文件
const languageFiles = fs.readdirSync(localesDir).filter(file => 
  file.endsWith('.json') && file !== 'en-US.json'
);

// 比较函数，找出缺失的键
function compareObjects(enObj, langObj, path = '') {
  const missing = [];
  
  // 检查enObj中的所有键
  for (const key in enObj) {
    if (Object.prototype.hasOwnProperty.call(enObj, key)) {
      const currentPath = path ? `${path}.${key}` : key;
      
      // 如果langObj中没有这个键
      if (!Object.prototype.hasOwnProperty.call(langObj, key)) {
        missing.push({
          path: currentPath,
          value: enObj[key]
        });
      } 
      // 如果是对象且不是数组，递归比较
      else if (typeof enObj[key] === 'object' && enObj[key] !== null && !Array.isArray(enObj[key])) {
        const nestedMissing = compareObjects(enObj[key], langObj[key], currentPath);
        missing.push(...nestedMissing);
      }
    }
  }
  
  return missing;
}

// 检查未翻译的字符串
function checkUntranslated(enObj, langObj, path = '') {
  const untranslated = [];
  
  for (const key in enObj) {
    if (Object.prototype.hasOwnProperty.call(enObj, key) && Object.prototype.hasOwnProperty.call(langObj, key)) {
      const currentPath = path ? `${path}.${key}` : key;
      
      if (typeof enObj[key] === 'string' && enObj[key] === langObj[key]) {
        untranslated.push({
          path: currentPath,
          value: enObj[key]
        });
      } else if (typeof enObj[key] === 'object' && enObj[key] !== null && !Array.isArray(enObj[key])) {
        const nestedUntranslated = checkUntranslated(enObj[key], langObj[key], currentPath);
        untranslated.push(...nestedUntranslated);
      }
    }
  }
  
  return untranslated;
}

// 处理每个语言文件
languageFiles.forEach(file => {
  const filePath = path.join(localesDir, file);
  const lang = JSON.parse(fs.readFileSync(filePath, 'utf8'));
  
  // 找出缺失的键
  const missingKeys = compareObjects(enUS, lang);
  
  if (missingKeys.length > 0) {
    console.log(`\nMissing keys in ${file}:`);
    console.log(missingKeys);
    console.log(`\nPlease translate these keys for ${file} and add them to the file.`);
  } else {
    console.log(`\n${file} has all required keys.`);
  }
  
  // 检查未翻译的字符串
  const untranslated = checkUntranslated(enUS, lang);
  if (untranslated.length > 0) {
    console.log(`\nUntranslated strings in ${file}:`);
    console.log(untranslated);
    console.log(`\nPlease translate these strings for ${file}.`);
  } else {
    console.log(`\n${file} has no untranslated strings.`);
  }
});

console.log('\nAll language files have been processed.');