import fs from 'fs';
import path from 'path';

class LocaleBuilder {
  constructor() {
    this.localesDir = path.resolve('src', 'i18n', 'locales');
    this.enUS = null;
    this.languageFiles = [];
  }

  loadBaseLocale() {
    const enUSPath = path.join(this.localesDir, 'en-US.json');
    this.enUS = JSON.parse(fs.readFileSync(enUSPath, 'utf8'));
    console.log(`Loaded base locale: en-US.json`);
    return this;
  }

  loadLanguageFiles() {
    this.languageFiles = fs.readdirSync(this.localesDir)
      .filter(file => file.endsWith('.json') && file !== 'en-US.json');
    console.log(`Found ${this.languageFiles.length} language files`);
    return this;
  }

  findMissingKeys(enObj, langObj, path = '') {
    const missing = [];
    
    for (const key in enObj) {
      if (Object.prototype.hasOwnProperty.call(enObj, key)) {
        const currentPath = path ? `${path}.${key}` : key;
        
        if (!Object.prototype.hasOwnProperty.call(langObj, key)) {
          missing.push({
            path: currentPath,
            value: enObj[key]
          });
        } else if (typeof enObj[key] === 'object' && enObj[key] !== null && !Array.isArray(enObj[key])) {
          const nestedMissing = this.findMissingKeys(enObj[key], langObj[key], currentPath);
          missing.push(...nestedMissing);
        }
      }
    }
    
    return missing;
  }

  generateTranslationTemplate(missingKeys) {
    const template = {};
    
    missingKeys.forEach(item => {
      const keys = item.path.split('.');
      let current = template;
      
      for (let i = 0; i < keys.length - 1; i++) {
        const key = keys[i];
        if (!current[key]) {
          current[key] = {};
        }
        current = current[key];
      }
      
      current[keys[keys.length - 1]] = ''; // 空字符串作为翻译占位符
    });
    
    return template;
  }

  process() {
    this.languageFiles.forEach(file => {
      const filePath = path.join(this.localesDir, file);
      const lang = JSON.parse(fs.readFileSync(filePath, 'utf8'));
      
      const missingKeys = this.findMissingKeys(this.enUS, lang);
      
      if (missingKeys.length > 0) {
        console.log(`${file} is missing ${missingKeys.length} keys`);
        const template = this.generateTranslationTemplate(missingKeys);
        const templatePath = path.join(this.localesDir, `${file.replace('.json', '')}-translate.json`);
        
        fs.writeFileSync(templatePath, JSON.stringify(template, null, 2), 'utf8');
        console.log(`Generated ${file.replace('.json', '')}-translate.json`);
      } else {
        console.log(`${file} has all required keys`);
      }
    });
    
    return this;
  }

  build() {
    return this.loadBaseLocale()
      .loadLanguageFiles()
      .process();
  }
}

// 执行构建
new LocaleBuilder().build();
console.log('Translation templates generated successfully.');