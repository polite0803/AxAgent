import fs from 'fs';
import path from 'path';

// 读取英文基准文件
const enFilePath = './locales/en-US.json';
const enContent = JSON.parse(fs.readFileSync(enFilePath, 'utf8'));

// 要检查的语言文件
const languages = ['zh-CN', 'zh-TW', 'ar', 'de', 'es', 'fr', 'hi', 'ja', 'ko', 'ru'];

// 递归获取所有键
function getAllKeys(obj, prefix = '') {
  let keys = [];
  for (const [key, value] of Object.entries(obj)) {
    const fullKey = prefix ? `${prefix}.${key}` : key;
    if (typeof value === 'object' && value !== null && !Array.isArray(value)) {
      keys = [...keys, ...getAllKeys(value, fullKey)];
    } else {
      keys.push(fullKey);
    }
  }
  return keys;
}

// 获取对象中指定键的值
function getValueByKey(obj, key) {
  return key.split('.').reduce((acc, curr) => acc?.[curr], obj);
}

// 比较两个语言文件，找出缺失的键
function findMissingKeys(baseObj, targetObj) {
  const baseKeys = getAllKeys(baseObj);
  const targetKeys = getAllKeys(targetObj);
  return baseKeys.filter(key => !targetKeys.includes(key));
}

// 生成缺失的文案
function generateMissingTranslations(baseObj, missingKeys, lang) {
  const missing = {};
  missingKeys.forEach(key => {
    const value = getValueByKey(baseObj, key);
    // 简单的翻译逻辑，实际项目中可能需要使用专业的翻译API
    let translatedValue = value;
    
    // 这里可以添加针对不同语言的翻译逻辑
    switch (lang) {
      case 'zh-CN':
        // 这里可以添加中文翻译
        break;
      case 'zh-TW':
        // 这里可以添加繁体中文翻译
        break;
      case 'de':
        // 这里可以添加德语翻译
        break;
      case 'es':
        // 这里可以添加西班牙语翻译
        break;
      case 'fr':
        // 这里可以添加法语翻译
        break;
      case 'hi':
        // 这里可以添加印地语翻译
        break;
      case 'ja':
        // 这里可以添加日语翻译
        break;
      case 'ko':
        // 这里可以添加韩语翻译
        break;
      case 'ru':
        // 这里可以添加俄语翻译
        break;
      case 'ar':
        // 这里可以添加阿拉伯语翻译
        break;
    }
    
    // 将键路径转换为对象结构
    const keyParts = key.split('.');
    let current = missing;
    for (let i = 0; i < keyParts.length - 1; i++) {
      const part = keyParts[i];
      if (!current[part]) {
        current[part] = {};
      }
      current = current[part];
    }
    current[keyParts[keyParts.length - 1]] = translatedValue;
  });
  return missing;
}

// 合并对象
function mergeObjects(target, source) {
  for (const [key, value] of Object.entries(source)) {
    if (typeof value === 'object' && value !== null && !Array.isArray(value)) {
      if (!target[key] || typeof target[key] !== 'object' || Array.isArray(target[key])) {
        target[key] = {};
      }
      mergeObjects(target[key], value);
    } else {
      target[key] = value;
    }
  }
  return target;
}

// 处理每个语言文件
languages.forEach(lang => {
  // 保持语言代码的正确大小写，特别是zh-CN和zh-TW
  let filename = lang;
  // 对于zh-CN和zh-TW，保持大写的国家代码
  if (lang === 'zh-CN' || lang === 'zh-TW') {
    filename = lang;
  } else {
    filename = lang.toLowerCase();
  }
  const langFilePath = `./locales/${filename}.json`;
  
  if (fs.existsSync(langFilePath)) {
    const langContent = JSON.parse(fs.readFileSync(langFilePath, 'utf8'));
    const missingKeys = findMissingKeys(enContent, langContent);
    
    console.log(`Language: ${lang}`);
    console.log(`Missing keys: ${missingKeys.length}`);
    console.log(`Missing keys: ${missingKeys.join(', ')}`);
    console.log('');
    
    if (missingKeys.length > 0) {
      const missingTranslations = generateMissingTranslations(enContent, missingKeys, lang);
      const mergedContent = mergeObjects(langContent, missingTranslations);
      
      // 写入更新后的文件
      fs.writeFileSync(langFilePath, JSON.stringify(mergedContent, null, 2));
      console.log(`Updated ${lang} file with ${missingKeys.length} missing translations`);
    }
  } else {
    console.log(`Language file not found: ${langFilePath}`);
  }
  console.log('---');
});

console.log('Comparison completed!');