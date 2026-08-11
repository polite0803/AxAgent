const fs = require('fs');

const translations = {
  'ja.json': {
    title: 'メモリーグラフ',
    searchPlaceholder: 'エンティティまたはタグを検索...',
    filterByType: 'タイプでフィルタ',
    nodeCount: '{count} エンティティ',
    edgeCount: '{count} リレーション',
    refresh: 'リフレッシュ',
    entityTypes: 'エンティティタイプ',
    noData: 'データなし',
    emptyDescription: 'メモリーグラフデータがありません。会話を開始して構築してください。',
    nodeDetails: 'ノード詳細',
    mentionCount: '{count}回言及',
    backlinkCount: '{count}回参照',
    tags: 'タグ',
    relationships: 'リレーション'
  },
  'de.json': {
    title: 'Speichergraph',
    searchPlaceholder: 'Entitäten oder Tags durchsuchen...',
    filterByType: 'Nach Typ filtern',
    nodeCount: '{count} Entitäten',
    edgeCount: '{count} Beziehungen',
    refresh: 'Aktualisieren',
    entityTypes: 'Entitätstypen',
    noData: 'Keine Daten',
    emptyDescription: 'Noch keine Speichergraphdaten. Starten Sie ein Gespräch, um sie zu erstellen.',
    nodeDetails: 'Knotendetails',
    mentionCount: '{count} Mal erwähnt',
    backlinkCount: '{count} Mal referenziert',
    tags: 'Tags',
    relationships: 'Beziehungen'
  },
  'es.json': {
    title: 'Grafo de Memoria',
    searchPlaceholder: 'Buscar entidades o etiquetas...',
    filterByType: 'Filtrar por tipo',
    nodeCount: '{count} entidades',
    edgeCount: '{count} relaciones',
    refresh: 'Actualizar',
    entityTypes: 'Tipos de entidad',
    noData: 'Sin datos',
    emptyDescription: 'Aún no hay datos de grafo de memoria. Inicia una conversación para construirlo.',
    nodeDetails: 'Detalles del nodo',
    mentionCount: 'Mencionado {count} veces',
    backlinkCount: 'Referenciado {count} veces',
    tags: 'Etiquetas',
    relationships: 'Relaciones'
  },
  'fr.json': {
    title: 'Graphe mémoire',
    searchPlaceholder: 'Rechercher entités ou étiquettes...',
    filterByType: 'Filtrer par type',
    nodeCount: '{count} entités',
    edgeCount: '{count} relations',
    refresh: 'Actualiser',
    entityTypes: "Types d'entité",
    noData: 'Aucune donnée',
    emptyDescription: 'Pas encore de données de graphe mémoire. Commencez une conversation pour le créer.',
    nodeDetails: 'Détails du nœud',
    mentionCount: 'Mentionné {count} fois',
    backlinkCount: 'Référencé {count} fois',
    tags: 'Étiquettes',
    relationships: 'Relations'
  },
  'hi.json': {
    title: 'मेमोरी ग्राफ',
    searchPlaceholder: 'एंटिटी या टैग खोजें...',
    filterByType: 'प्रकार से फ़िल्टर करें',
    nodeCount: '{count} एंटिटी',
    edgeCount: '{count} संबंध',
    refresh: 'रीफ्रेश',
    entityTypes: 'एंटिटी प्रकार',
    noData: 'कोई डेटा नहीं',
    emptyDescription: 'अभी कोई मेमोरी ग्राफ डेटा नहीं है। इसे बनाने के लिए बातचीत शुरू करें।',
    nodeDetails: 'नोड विवरण',
    mentionCount: '{count} बार उल्लिखित',
    backlinkCount: '{count} बार संदर्भित',
    tags: 'टैग',
    relationships: 'संबंध'
  },
  'ko.json': {
    title: '메모리 그래프',
    searchPlaceholder: '엔티티 또는 태그 검색...',
    filterByType: '유형별 필터',
    nodeCount: '{count}개 엔티티',
    edgeCount: '{count}개 관계',
    refresh: '새로고침',
    entityTypes: '엔티티 유형',
    noData: '데이터 없음',
    emptyDescription: '아직 메모리 그래프 데이터가 없습니다. 대화를 시작하여 생성하세요.',
    nodeDetails: '노드 세부 정보',
    mentionCount: '{count}회 언급',
    backlinkCount: '{count}회 참조',
    tags: '태그',
    relationships: '관계'
  },
  'ru.json': {
    title: 'Граф памяти',
    searchPlaceholder: 'Поиск сущностей или тегов...',
    filterByType: 'Фильтр по типу',
    nodeCount: '{count} сущностей',
    edgeCount: '{count} связей',
    refresh: 'Обновить',
    entityTypes: 'Типы сущностей',
    noData: 'Нет данных',
    emptyDescription: 'Данных графа памяти пока нет. Начните разговор, чтобы создать их.',
    nodeDetails: 'Детали узла',
    mentionCount: 'Упоминается {count} раз',
    backlinkCount: 'Ссылается {count} раз',
    tags: 'Теги',
    relationships: 'Связи'
  },
  'zh-TW.json': {
    title: '記憶圖譜',
    searchPlaceholder: '搜尋實體或標籤...',
    filterByType: '依類型篩選',
    nodeCount: '{count} 個實體',
    edgeCount: '{count} 條關係',
    refresh: '重新整理',
    entityTypes: '實體類型',
    noData: '暫無資料',
    emptyDescription: '尚未有記憶圖譜資料，開始對話後會自動建立。',
    nodeDetails: '節點詳情',
    mentionCount: '提及 {count} 次',
    backlinkCount: '被引用 {count} 次',
    tags: '標籤',
    relationships: '關聯關係'
  },
  'ar.json': {
    title: 'رسم بياني للذاكرة',
    searchPlaceholder: 'البحث عن الكيانات أو الوسوم...',
    filterByType: 'تصفية حسب النوع',
    nodeCount: '{count} كيان',
    edgeCount: '{count} علاقة',
    refresh: 'تحديث',
    entityTypes: 'أنواع الكيانات',
    noData: 'لا توجد بيانات',
    emptyDescription: 'لا توجد بيانات رسم بياني للذاكرة بعد. ابدأ محادثة لبنائها.',
    nodeDetails: 'تفاصيل العقدة',
    mentionCount: 'تم ذكره {count} مرة',
    backlinkCount: 'تمت الإشارة إليه {count} مرة',
    tags: 'الوسوم',
    relationships: 'العلاقات'
  }
};

for (const [file, graph] of Object.entries(translations)) {
  const path = 'src/i18n/locales/' + file;
  try {
    const data = JSON.parse(fs.readFileSync(path, 'utf8'));
    data.memory = {
      ...data.memory,
      graph
    };
    fs.writeFileSync(path, JSON.stringify(data, null, 2) + '\n');
    console.log('Updated: ' + file);
  } catch (e) {
    console.log('Error updating ' + file + ': ' + e.message);
  }
}
