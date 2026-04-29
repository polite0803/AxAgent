
import fs from 'fs';

// 各个语言的 skills.marketplace 翻译
const translations = {
  'de.json': {
    title: 'Marktplatz',
    convertToWorkflow: 'In Workflow umwandeln',
    skillsMdNotFound: 'SKILL.md nicht gefunden (nicht im main oder master Branch)',
    skillsMdFetchFailed: 'Fehler beim Abrufen von SKILL.md'
  },
  'es.json': {
    title: 'Mercado',
    convertToWorkflow: 'Convertir a flujo de trabajo',
    skillsMdNotFound: 'SKILL.md no encontrado (no encontrado en la rama main o master)',
    skillsMdFetchFailed: 'Error al obtener SKILL.md'
  },
  'fr.json': {
    title: 'Marketplace',
    convertToWorkflow: 'Convertir en workflow',
    skillsMdNotFound: 'SKILL.md non trouvé (pas trouvé dans la branche main ou master)',
    skillsMdFetchFailed: 'Échec de la récupération de SKILL.md'
  },
  'hi.json': {
    title: 'मार्केटप्लेस',
    convertToWorkflow: 'वर्कफ़्लो में बदलें',
    skillsMdNotFound: 'SKILL.md नहीं मिला (main या master शाखा में नहीं मिला)',
    skillsMdFetchFailed: 'SKILL.md प्राप्त करने में विफल'
  },
  'ja.json': {
    title: 'マーケットプレイス',
    convertToWorkflow: 'ワークフローに変換',
    skillsMdNotFound: 'SKILL.mdが見つかりません（mainまたはmasterブランチにありません）',
    skillsMdFetchFailed: 'SKILL.mdの取得に失敗しました'
  }
};

// 处理每个文件
const langs = ['de.json', 'es.json', 'fr.json', 'hi.json', 'ja.json'];

for (const file of langs) {
  const langPath = 'src/i18n/locales/' + file;
  let lang;
  try {
    lang = JSON.parse(fs.readFileSync(langPath, 'utf8'));
  } catch (e) {
    console.error(`Error parsing ${file}: ${e}`);
    continue;
  }

  const t = translations[file];
  if (!t) {
    continue;
  }

  // 检查 skills.marketplace 是否是字符串，如果是则转换为对象
  if (lang.skills && typeof lang.skills.marketplace === 'string') {
    // 保存现有的 title 值
    const existingTitle = lang.skills.marketplace;
    lang.skills.marketplace = {
      title: existingTitle,
      ...t
    };
    console.log(`${file}: Converted skills.marketplace to object`);
  }

  // 保存文件
  fs.writeFileSync(langPath, JSON.stringify(lang, null, 2) + '\n');
}

console.log('Done.');
