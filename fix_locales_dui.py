# -*- coding: utf-8 -*-
import json

base = 'd:/OneManager/AxAgent/src/i18n/locales'

# Additional keys to add (missing from ALL locales including zh-CN)
new_keys = {
    'zh-CN.json': {'componentType': '组件类型', 'addProp': '添加属性',
                   'requiredProps': '必填属性', 'noProps': '暂无属性',
                   'addChild': '添加子节点', 'noChildren': '暂无子节点'},
    'zh-TW.json': {'componentType': '元件類型', 'addProp': '新增屬性',
                   'requiredProps': '必填屬性', 'noProps': '暫無屬性',
                   'addChild': '新增子節點', 'noChildren': '暫無子節點'},
    'en-US.json': {'componentType': 'Component Type', 'addProp': 'Add Prop',
                   'requiredProps': 'Required Props', 'noProps': 'No Props',
                   'addChild': 'Add Child', 'noChildren': 'No Children'},
    'ja.json': {'componentType': 'コンポーネントタイプ', 'addProp': 'プロパティを追加',
                'requiredProps': '必須プロパティ', 'noProps': 'プロパティなし',
                'addChild': '子を追加', 'noChildren': '子なし'},
    'ko.json': {'componentType': '컴포넌트 유형', 'addProp': '속성 추가',
                'requiredProps': '필수 속성', 'noProps': '속성 없음',
                'addChild': '자식 추가', 'noChildren': '자식 없음'},
    'de.json': {'componentType': 'Komponententyp', 'addProp': 'Eigenschaft hinzufügen',
                'requiredProps': 'Erforderliche Eigenschaften', 'noProps': 'Keine Eigenschaften',
                'addChild': 'Kind hinzufügen', 'noChildren': 'Keine Kinder'},
    'fr.json': {'componentType': 'Type de composant', 'addProp': 'Ajouter une propriété',
                'requiredProps': 'Propriétés requises', 'noProps': 'Aucune propriété',
                'addChild': 'Ajouter un enfant', 'noChildren': 'Aucun enfant'},
    'es.json': {'componentType': 'Tipo de componente', 'addProp': 'Agregar propiedad',
                'requiredProps': 'Propiedades requeridas', 'noProps': 'Sin propiedades',
                'addChild': 'Agregar hijo', 'noChildren': 'Sin hijos'},
    'ru.json': {'componentType': 'Тип компонента', 'addProp': 'Добавить свойство',
                'requiredProps': 'Обязательные свойства', 'noProps': 'Нет свойств',
                'addChild': 'Добавить дочерний', 'noChildren': 'Нет дочерних'},
    'ar.json': {'componentType': 'نوع المكون', 'addProp': 'إضافة خاصية',
                'requiredProps': 'الخصائص المطلوبة', 'noProps': 'لا توجد خصائص',
                'addChild': 'إضافة عنصر فرعي', 'noChildren': 'لا توجد عناصر فرعية'},
    'hi.json': {'componentType': 'कंपोनेंट प्रकार', 'addProp': 'गुण जोड़ें',
                'requiredProps': 'आवश्यक गुण', 'noProps': 'कोई गुण नहीं',
                'addChild': 'चाइल्ड जोड़ें', 'noChildren': 'कोई चाइल्ड नहीं'},
}

# Full form translations for each non-zh-CN locale (they were all missing)
form_translations = {
    'en-US.json': {
        'title': 'Dynamic UI Manager',
        'subtitle': 'Create, manage and preview dynamic UI components with NL generation support',
        'createNew': 'New UI', 'schemaList': 'UI List', 'preview': 'Preview',
        'noSchemas': 'No UIs yet. Click the top-right button to create one',
        'selectToPreview': 'Select a UI from the left to preview',
        'createSchema': 'New UI', 'editSchema': 'Edit UI',
        'schemaTitle': 'Title', 'titlePlaceholder': 'Enter UI title',
        'titleRequired': 'Please enter a title', 'description': 'Description',
        'descPlaceholder': 'Enter UI description', 'category': 'Category',
        'tags': 'Tags', 'tagsPlaceholder': 'Enter tags and press Enter',
        'jsonEditor': 'JSON Schema', 'jsonPlaceholder': 'Enter JSON conforming to UISchema spec...',
        'parseError': 'Parse Error', 'schemaValid': 'Schema validated',
        'schemaRequired': 'Schema cannot be empty', 'invalidJson': 'Invalid JSON format',
        'invalidSchema': 'Schema parsing failed', 'createSuccess': 'Created successfully',
        'updateSuccess': 'Updated successfully', 'deleteSuccess': 'Deleted successfully',
        'confirmDelete': 'Are you sure you want to delete this UI?',
        'builtin': 'Built-in', 'noDescription': 'No description',
        'generateWithNL': 'Generate with NL', 'nlInputPlaceholder': 'Describe the UI you want...',
        'generating': 'Generating...', 'generateSuccess': 'Generated successfully. Please review and save',
        # Add category labels
        'catForm': 'Form', 'catDashboard': 'Dashboard', 'catReport': 'Report', 'catCustom': 'Custom',
    },
    'ja.json': {
        'title': '動的UI管理',
        'subtitle': '動的UIコンポーネントの作成、管理、プレビュー（自然言語生成対応）',
        'createNew': '新規UI', 'schemaList': 'UI一覧', 'preview': 'プレビュー',
        'noSchemas': 'UIがありません。右上のボタンをクリックして作成',
        'selectToPreview': '左側のUIを選択してプレビュー',
        'createSchema': '新規UI', 'editSchema': 'UI編集',
        'schemaTitle': 'タイトル', 'titlePlaceholder': 'UIタイトルを入力',
        'titleRequired': 'タイトルを入力してください', 'description': '説明',
        'descPlaceholder': 'UIの説明を入力', 'category': 'カテゴリ',
        'tags': 'タグ', 'tagsPlaceholder': 'タグを入力してEnter',
        'jsonEditor': 'JSONスキーマ', 'jsonPlaceholder': 'UISchema仕様に準拠したJSONを入力...',
        'parseError': '解析エラー', 'schemaValid': 'スキーマ検証済み',
        'schemaRequired': 'スキーマは空にできません', 'invalidJson': '無効なJSON形式',
        'invalidSchema': 'スキーマ解析に失敗しました', 'createSuccess': '作成成功',
        'updateSuccess': '更新成功', 'deleteSuccess': '削除成功',
        'confirmDelete': 'このUIを削除してもよろしいですか？',
        'builtin': '組み込み', 'noDescription': '説明なし',
        'generateWithNL': '自然言語生成', 'nlInputPlaceholder': '希望するUIを説明してください...',
        'generating': '生成中...', 'generateSuccess': '生成成功。確認して保存してください',
        'catForm': 'フォーム', 'catDashboard': 'ダッシュボード', 'catReport': 'レポート', 'catCustom': 'カスタム',
    },
    'ko.json': {
        'title': '동적 UI 관리',
        'subtitle': 'NL 생성 지원으로 동적 UI 구성 요소 생성, 관리 및 미리보기',
        'createNew': '새 UI', 'schemaList': 'UI 목록', 'preview': '미리보기',
        'noSchemas': 'UI가 없습니다. 오른쪽 상단 버튼을 클릭하여 생성',
        'selectToPreview': '왼쪽에서 UI를 선택하여 미리보기',
        'createSchema': '새 UI', 'editSchema': 'UI 편집',
        'schemaTitle': '제목', 'titlePlaceholder': 'UI 제목 입력',
        'titleRequired': '제목을 입력하세요', 'description': '설명',
        'descPlaceholder': 'UI 설명 입력', 'category': '카테고리',
        'tags': '태그', 'tagsPlaceholder': '태그를 입력하고 Enter',
        'jsonEditor': 'JSON 스키마', 'jsonPlaceholder': 'UISchema 사양에 맞는 JSON 입력...',
        'parseError': '구문 분석 오류', 'schemaValid': '스키마 검증 완료',
        'schemaRequired': '스키마는 비워둘 수 없습니다', 'invalidJson': '잘못된 JSON 형식',
        'invalidSchema': '스키마 분석 실패', 'createSuccess': '성공적으로 생성됨',
        'updateSuccess': '성공적으로 업데이트됨', 'deleteSuccess': '성공적으로 삭제됨',
        'confirmDelete': '이 UI를 삭제하시겠습니까?',
        'builtin': '내장', 'noDescription': '설명 없음',
        'generateWithNL': 'NL로 생성', 'nlInputPlaceholder': '원하는 UI를 설명하세요...',
        'generating': '생성 중...', 'generateSuccess': '성공적으로 생성되었습니다. 확인 후 저장하세요',
        'catForm': '양식', 'catDashboard': '대시보드', 'catReport': '리포트', 'catCustom': '사용자 정의',
    },
    'zh-TW.json': {
        'title': '動態UI管理',
        'subtitle': '創建、管理和預覽動態UI元件，支援自然語言生成',
        'createNew': '新建UI', 'schemaList': 'UI列表', 'preview': '預覽',
        'noSchemas': '暫無UI，點擊右上角新建',
        'selectToPreview': '選擇左側UI進行預覽',
        'createSchema': '新建UI', 'editSchema': '編輯UI',
        'schemaTitle': '標題', 'titlePlaceholder': '請輸入UI標題',
        'titleRequired': '請輸入標題', 'description': '描述',
        'descPlaceholder': '請輸入UI描述', 'category': '分類',
        'tags': '標籤', 'tagsPlaceholder': '輸入標籤後回車',
        'jsonEditor': 'JSON Schema', 'jsonPlaceholder': '請輸入符合UISchema規範的JSON...',
        'parseError': '解析錯誤', 'schemaValid': 'Schema 驗證通過',
        'schemaRequired': 'Schema不能為空', 'invalidJson': 'JSON格式無效',
        'invalidSchema': 'Schema解析失敗', 'createSuccess': '創建成功',
        'updateSuccess': '更新成功', 'deleteSuccess': '刪除成功',
        'confirmDelete': '確定要刪除此UI嗎？',
        'builtin': '內置', 'noDescription': '暫無描述',
        'generateWithNL': '自然語言生成', 'nlInputPlaceholder': '描述你想要的UI...',
        'generating': '正在生成...', 'generateSuccess': '生成成功，請檢查後保存',
        'catForm': '表單', 'catDashboard': '儀表板', 'catReport': '報表', 'catCustom': '自定義',
    },
}

# For locales I don't have custom translations, also add cat* labels
cat_fallbacks = {
    'ar.json': {'catForm': 'نموذج', 'catDashboard': 'لوحة القيادة', 'catReport': 'تقرير', 'catCustom': 'مخصص'},
    'de.json': {'catForm': 'Formular', 'catDashboard': 'Dashboard', 'catReport': 'Bericht', 'catCustom': 'Benutzerdefiniert'},
    'es.json': {'catForm': 'Formulario', 'catDashboard': 'Panel', 'catReport': 'Informe', 'catCustom': 'Personalizado'},
    'fr.json': {'catForm': 'Formulaire', 'catDashboard': 'Tableau de bord', 'catReport': 'Rapport', 'catCustom': 'Personnalisé'},
    'hi.json': {'catForm': 'फ़ॉर्म', 'catDashboard': 'डैशबोर्ड', 'catReport': 'रिपोर्ट', 'catCustom': 'कस्टम'},
    'ru.json': {'catForm': 'Форма', 'catDashboard': 'Панель', 'catReport': 'Отчет', 'catCustom': 'Пользовательский'},
}

cat_zh_cn = {'catForm': '表单', 'catDashboard': '仪表盘', 'catReport': '报表', 'catCustom': '自定义'}

# Process each locale file
import os
for fname in sorted(os.listdir(base)):
    if not fname.endswith('.json'):
        continue
    with open(f'{base}/{fname}', 'r', encoding='utf-8') as f:
        data = json.load(f)

    if 'dynamicUIManager' not in data:
        data['dynamicUIManager'] = {}

    dui = data['dynamicUIManager']
    extras = {}

    # Add zh-CN category labels
    if fname == 'zh-CN.json':
        extras.update(cat_zh_cn)

    # Add form translations for non-zh-CN locales that are missing
    if fname in form_translations:
        extras.update(form_translations[fname])

    # Add cat labels for specific locales
    if fname in cat_fallbacks:
        extras.update(cat_fallbacks[fname])

    # Add new componentType keys
    if fname in new_keys:
        extras.update(new_keys[fname])

    # Apply all updates (don't overwrite existing defaults key)
    for k, v in extras.items():
        if k not in dui:
            dui[k] = v

    with open(f'{base}/{fname}', 'w', encoding='utf-8') as f:
        json.dump(data, f, ensure_ascii=False, indent=2)
        f.write('\n')
    print(f'OK {fname}')

print('Done!')
