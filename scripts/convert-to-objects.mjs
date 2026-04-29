
import fs from 'fs';

// 读取英文原文
const en = JSON.parse(fs.readFileSync('src/i18n/locales/en-US.json', 'utf8'));

// 各个语言的翻译
const translations = {
  'ar.json': {
    'settings.theme': {
      title: 'مدير السمات',
      description: 'إدارة سمات التطبيق ومخططات الألوان',
      builtInThemes: 'السمات المضمنة',
      customThemes: 'السمات المخصصة',
      deleteConfirm: 'حذف هذه السمة؟',
      deleted: 'تم حذف السمة',
      exported: 'تم تصدير السمة',
      import: 'استيراد',
      importTitle: 'استيراد سمة',
      imported: 'تم استيراد السمة',
      invalidYaml: 'تنسيق YAML غير صالح',
      refresh: 'تحديث',
      yamlContent: 'محتوى YAML',
      yamlRequired: 'محتوى YAML مطلوب'
    },
    'settings.shortcuts': {
      title: 'الاختصارات',
      description: 'تخصيص ربطات اختصار لوحة المفاتيح',
      reset: 'إعادة تعيين',
      resetAll: 'إعادة تعيين الكل',
      saved: 'تم الحفظ'
    },
    'style.dimensions': {
      label: 'الأبعاد',
      abstraction: 'التجريد',
      commentRatio: 'نسبة التعليقات',
      density: 'الكثافة',
      explanationLength: 'طول الشرح',
      formality: 'الرسمية',
      naming: 'التسمية',
      structure: 'البنية',
      technicalDepth: 'العمق الفني'
    }
  },
  'de.json': {
    'settings.theme': {
      title: 'Themen-Manager',
      description: 'Anwendungsthemen und Farbschemas verwalten',
      builtInThemes: 'Eingebaute Themen',
      customThemes: 'Benutzerdefinierte Themen',
      deleteConfirm: 'Dieses Thema löschen?',
      deleted: 'Thema gelöscht',
      exported: 'Thema exportiert',
      import: 'Importieren',
      importTitle: 'Thema importieren',
      imported: 'Thema importiert',
      invalidYaml: 'Ungültiges YAML-Format',
      refresh: 'Aktualisieren',
      yamlContent: 'YAML-Inhalt',
      yamlRequired: 'YAML-Inhalt erforderlich'
    },
    'settings.shortcuts': {
      title: 'Tastenkürzel',
      description: 'Tastenkürzel zuweisen',
      reset: 'Zurücksetzen',
      resetAll: 'Alle zurücksetzen',
      saved: 'Gespeichert'
    },
    'skills.marketplace': {
      title: 'Marketplace',
      convertToWorkflow: 'In Workflow umwandeln',
      skillsMdNotFound: 'SKILL.md nicht gefunden (nicht im main oder master Branch)',
      skillsMdFetchFailed: 'Fehler beim Abrufen von SKILL.md'
    },
    'style.dimensions': {
      label: 'Dimensionen',
      abstraction: 'Abstraktion',
      commentRatio: 'Kommentarrate',
      density: 'Dichte',
      explanationLength: 'Erklärungslänge',
      formality: 'Formalität',
      naming: 'Benennung',
      structure: 'Struktur',
      technicalDepth: 'Technische Tiefe'
    }
  },
  'es.json': {
    'settings.theme': {
      title: 'Gestor de temas',
      description: 'Gestionar temas de aplicación y esquemas de color',
      builtInThemes: 'Temas integrados',
      customThemes: 'Temas personalizados',
      deleteConfirm: '¿Eliminar este tema?',
      deleted: 'Tema eliminado',
      exported: 'Tema exportado',
      import: 'Importar',
      importTitle: 'Importar tema',
      imported: 'Tema importado',
      invalidYaml: 'Formato YAML inválido',
      refresh: 'Actualizar',
      yamlContent: 'Contenido YAML',
      yamlRequired: 'Contenido YAML es requerido'
    },
    'settings.shortcuts': {
      title: 'Atajos',
      description: 'Personalizar enlaces de atajos de teclado',
      reset: 'Restablecer',
      resetAll: 'Restablecer todo',
      saved: 'Guardado'
    },
    'skills.marketplace': {
      title: 'Marketplace',
      convertToWorkflow: 'Convertir a flujo de trabajo',
      skillsMdNotFound: 'SKILL.md no encontrado (no encontrado en la rama main o master)',
      skillsMdFetchFailed: 'Error al obtener SKILL.md'
    },
    'style.dimensions': {
      label: 'Dimensiones',
      abstraction: 'Abstracción',
      commentRatio: 'Relación de comentarios',
      density: 'Densidad',
      explanationLength: 'Longitud de explicación',
      formality: 'Formalidad',
      naming: 'Nombramiento',
      structure: 'Estructura',
      technicalDepth: 'Profundidad técnica'
    }
  },
  'fr.json': {
    'settings.theme': {
      title: 'Gestionnaire de thème',
      description: "Gérer les thèmes d'application et les schémas de couleur",
      builtInThemes: 'Thèmes intégrés',
      customThemes: 'Thèmes personnalisés',
      deleteConfirm: 'Supprimer ce thème?',
      deleted: 'Thème supprimé',
      exported: 'Thème exporté',
      import: 'Importer',
      importTitle: 'Importer un thème',
      imported: 'Thème importé',
      invalidYaml: 'Format YAML non valide',
      refresh: 'Actualiser',
      yamlContent: 'Contenu YAML',
      yamlRequired: 'Le contenu YAML est requis'
    },
    'settings.shortcuts': {
      title: 'Raccourcis',
      description: 'Personnaliser les raccourcis clavier',
      reset: 'Réinitialiser',
      resetAll: 'Tout réinitialiser',
      saved: 'Enregistré'
    },
    'skills.marketplace': {
      title: 'Marketplace',
      convertToWorkflow: 'Convertir en workflow',
      skillsMdNotFound: 'SKILL.md non trouvé (pas trouvé dans la branche main ou master)',
      skillsMdFetchFailed: 'Échec de la récupération de SKILL.md'
    },
    'style.dimensions': {
      label: 'Dimensions',
      abstraction: 'Abstraction',
      commentRatio: 'Ratio de commentaires',
      density: 'Densité',
      explanationLength: "Longueur de l'explication",
      formality: 'Formalité',
      naming: 'Nomination',
      structure: 'Structure',
      technicalDepth: 'Profondeur technique'
    }
  },
  'hi.json': {
    'settings.theme': {
      title: 'थीम प्रबंधक',
      description: 'एप्लिकेशन थीम और रंग योजनाएं प्रबंधित करें',
      builtInThemes: 'अंतर्निहित थीम',
      customThemes: 'कस्टम थीम',
      deleteConfirm: 'इस थीम को हटाएं?',
      deleted: 'थीम हटाई गई',
      exported: 'थीम निर्यात की गई',
      import: 'आयात करें',
      importTitle: 'थीम आयात करें',
      imported: 'थीम आयात की गई',
      invalidYaml: 'अमान्य YAML प्रारूप',
      refresh: 'रिफ्रेश',
      yamlContent: 'YAML सामग्री',
      yamlRequired: 'YAML सामग्री आवश्यक है'
    },
    'settings.shortcuts': {
      title: 'शॉर्टकट',
      description: 'कीबोर्ड शॉर्टकट बाइंडिंग अनुकूलित करें',
      reset: 'रीसेट',
      resetAll: 'सभी रीसेट करें',
      saved: 'सहेजा गया'
    },
    'skills.marketplace': {
      title: 'Marketplace',
      convertToWorkflow: 'वर्कफ़्लो में बदलें',
      skillsMdNotFound: 'SKILL.md नहीं मिला (main या master शाखा में नहीं मिला)',
      skillsMdFetchFailed: 'SKILL.md प्राप्त करने में विफल'
    },
    'style.dimensions': {
      label: 'आयाम',
      abstraction: 'सार',
      commentRatio: 'टिप्पणी अनुपात',
      density: 'घनत्व',
      explanationLength: 'विवरण लंबाई',
      formality: 'औपचारिकता',
      naming: 'नामकरण',
      structure: 'संरचना',
      technicalDepth: 'तकनीकी गहराई'
    }
  },
  'ja.json': {
    'settings.theme': {
      title: 'テーママネージャー',
      description: 'アプリケーションのテーマとカラースキームを管理',
      builtInThemes: '組み込みテーマ',
      customThemes: 'カスタムテーマ',
      deleteConfirm: 'このテーマを削除しますか？',
      deleted: 'テーマを削除しました',
      exported: 'テーマをエクスポートしました',
      import: 'インポート',
      importTitle: 'テーマをインポート',
      imported: 'テーマをインポートしました',
      invalidYaml: '無効なYAML形式',
      refresh: '更新',
      yamlContent: 'YAMLコンテンツ',
      yamlRequired: 'YAMLコンテンツが必要です'
    },
    'settings.shortcuts': {
      title: 'ショートカット',
      description: 'キーボードショートカットバインディングをカスタマイズ',
      reset: 'リセット',
      resetAll: 'すべてリセット',
      saved: '保存しました'
    },
    'skills.marketplace': {
      title: 'マーケットプレイス',
      convertToWorkflow: 'ワークフローに変換',
      skillsMdNotFound: 'SKILL.mdが見つかりません（mainまたはmasterブランチにありません）',
      skillsMdFetchFailed: 'SKILL.mdの取得に失敗しました'
    },
    'style.dimensions': {
      label: '次元',
      abstraction: '抽象化',
      commentRatio: 'コメント率',
      density: '密度',
      explanationLength: '説明の長さ',
      formality: 'フォーマリティ',
      naming: '命名',
      structure: '構造',
      technicalDepth: '技術的深さ'
    }
  },
  'ko.json': {
    'settings.theme': {
      title: '테마 관리자',
      description: '애플리케이션 테마 및 색상 구성표 관리',
      builtInThemes: '내장 테마',
      customThemes: '사용자 정의 테마',
      deleteConfirm: '이 테마를 삭제하시겠습니까?',
      deleted: '테마가 삭제되었습니다',
      exported: '테마가 내보내졌습니다',
      import: '가져오기',
      importTitle: '테마 가져오기',
      imported: '테마가 가져와졌습니다',
      invalidYaml: '잘못된 YAML 형식',
      refresh: '새로 고침',
      yamlContent: 'YAML 내용',
      yamlRequired: 'YAML 내용이 필요합니다'
    },
    'settings.shortcuts': {
      title: '단축키',
      description: '키보드 단축키 바인딩 사용자 지정',
      reset: '초기화',
      resetAll: '모두 초기화',
      saved: '저장됨'
    },
    'style.dimensions': {
      label: '차원',
      abstraction: '추상화',
      commentRatio: '댓글 비율',
      density: '밀도',
      explanationLength: '설명 길이',
      formality: '형식',
      naming: '명명',
      structure: '구조',
      technicalDepth: '기술적 깊이'
    }
  },
  'ru.json': {
    'settings.theme': {
      title: 'Менеджер тем',
      description: 'Управление темами приложения и цветовыми схемами',
      builtInThemes: 'Встроенные темы',
      customThemes: 'Пользовательские темы',
      deleteConfirm: 'Удалить эту тему?',
      deleted: 'Тема удалена',
      exported: 'Тема экспортирована',
      import: 'Импорт',
      importTitle: 'Импорт темы',
      imported: 'Тема импортирована',
      invalidYaml: 'Неверный формат YAML',
      refresh: 'Обновить',
      yamlContent: 'Содержимое YAML',
      yamlRequired: 'Содержимое YAML обязательно'
    },
    'settings.shortcuts': {
      title: 'Сочетания клавиш',
      description: 'Настроить сочетания клавиш',
      reset: 'Сбросить',
      resetAll: 'Сбросить всё',
      saved: 'Сохранено'
    },
    'style.dimensions': {
      label: 'Размерности',
      abstraction: 'Абстракция',
      commentRatio: 'Коэффициент комментариев',
      density: 'Плотность',
      explanationLength: 'Длина объяснения',
      formality: 'Формальность',
      naming: 'Именование',
      structure: 'Структура',
      technicalDepth: 'Техническая глубина'
    }
  },
  'zh-TW.json': {
    'settings.theme': {
      title: '主題管理',
      description: '管理應用主題和色彩方案',
      builtInThemes: '內建主題',
      customThemes: '自訂主題',
      deleteConfirm: '確定刪除此主題？',
      deleted: '主題已刪除',
      exported: '主題已匯出',
      import: '匯入',
      importTitle: '匯入主題',
      imported: '主題已匯入',
      invalidYaml: '無效的 YAML 格式',
      refresh: '重新整理',
      yamlContent: 'YAML 內容',
      yamlRequired: 'YAML 內容為必填'
    },
    'settings.shortcuts': {
      title: '快捷鍵',
      description: '自訂鍵盤快捷鍵綁定',
      reset: '重設',
      resetAll: '重設所有',
      saved: '已儲存'
    },
    'style.dimensions': {
      label: '維度',
      abstraction: '抽象化',
      commentRatio: '註解比例',
      density: '密度',
      explanationLength: '說明長度',
      formality: '正式程度',
      naming: '命名',
      structure: '結構',
      technicalDepth: '技術深度'
    }
  }
};

// 处理每个文件
const langs = ['ar.json', 'de.json', 'es.json', 'fr.json', 'hi.json', 'ja.json', 'ko.json', 'ru.json', 'zh-TW.json'];

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

  // 处理 settings.theme
  if (t['settings.theme']) {
    const existingValue = lang.settings?.theme;
    if (typeof existingValue === 'string') {
      lang.settings.theme = {
        label: existingValue,
        ...t['settings.theme']
      };
      console.log(`${file}: Converted settings.theme to object`);
    }
  }

  // 处理 settings.shortcuts
  if (t['settings.shortcuts']) {
    const existingValue = lang.settings?.shortcuts;
    if (typeof existingValue === 'string') {
      lang.settings.shortcuts = {
        label: existingValue,
        ...t['settings.shortcuts']
      };
      console.log(`${file}: Converted settings.shortcuts to object`);
    }
  }

  // 处理 skills.marketplace
  if (t['skills.marketplace'] && !lang.skills?.marketplace) {
    if (!lang.skills) lang.skills = {};
    lang.skills.marketplace = t['skills.marketplace'];
    console.log(`${file}: Added skills.marketplace`);
  }

  // 处理 style.dimensions
  if (t['style.dimensions']) {
    const existingValue = lang.style?.dimensions;
    if (typeof existingValue === 'string') {
      lang.style.dimensions = {
        label: existingValue,
        ...t['style.dimensions']
      };
      console.log(`${file}: Converted style.dimensions to object`);
    }
  }

  // 保存文件
  fs.writeFileSync(langPath, JSON.stringify(lang, null, 2) + '\n');
}

console.log('Done.');
