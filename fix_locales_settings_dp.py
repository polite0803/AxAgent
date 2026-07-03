# -*- coding: utf-8 -*-
"""Add missing settings.dynamicPages keys to all 11 locale files."""
import json

base = 'd:/OneManager/AxAgent/src/i18n/locales'

# Keys to add with zh-CN translations
missing_keys_cn = {
    'editPage': '编辑页面',
    'visualEdit': '可视化编辑',
    'aiEdit': 'AI 编辑',
    'currentSchema': '当前 Schema',
    'childrenCount': '个子节点',
    'aiGenerateInstruction': '描述你想要创建的页面功能',
    'editInstruction': '描述你想要修改的内容',
    'aiEditPlaceholder': '描述修改内容，例如：添加一个搜索框、改为两列布局...',
    'schemaPreview': 'Schema 预览',
    'jsonEdit': 'JSON 编辑',
}

# English translations
missing_keys_en = {
    'editPage': 'Edit Page',
    'visualEdit': 'Visual Edit',
    'aiEdit': 'AI Edit',
    'currentSchema': 'Current Schema',
    'childrenCount': 'children',
    'aiGenerateInstruction': 'Describe the page you want to create',
    'editInstruction': 'Describe what you want to change',
    'aiEditPlaceholder': 'Describe changes, e.g.: add a search box, change to two-column layout...',
    'schemaPreview': 'Schema Preview',
    'jsonEdit': 'JSON Edit',
}

translations = {
    'ar.json': {
        'editPage': 'تعديل الصفحة', 'visualEdit': 'تحرير بصري', 'aiEdit': 'تعديل بالذكاء الاصطناعي',
        'currentSchema': 'المخطط الحالي', 'childrenCount': 'عناصر فرعية',
        'aiGenerateInstruction': 'صف الصفحة التي تريد إنشاءها',
        'editInstruction': 'صف ما تريد تعديله',
        'aiEditPlaceholder': 'صف التعديلات، مثال: أضف مربع بحث، غير إلى تخطيط عمودين...',
        'schemaPreview': 'معاينة المخطط', 'jsonEdit': 'تعديل JSON',
    },
    'de.json': {
        'editPage': 'Seite bearbeiten', 'visualEdit': 'Visuelle Bearbeitung', 'aiEdit': 'KI-Bearbeitung',
        'currentSchema': 'Aktuelles Schema', 'childrenCount': 'Kind-Elemente',
        'aiGenerateInstruction': 'Beschreibe die gewünschte Seite',
        'editInstruction': 'Beschreibe, was geändert werden soll',
        'aiEditPlaceholder': 'Änderungen beschreiben, z.B.: Suchfeld hinzufügen, auf Zweispaltenlayout umstellen...',
        'schemaPreview': 'Schema-Vorschau', 'jsonEdit': 'JSON-Bearbeitung',
    },
    'en-US.json': missing_keys_en,
    'es.json': {
        'editPage': 'Editar página', 'visualEdit': 'Edición visual', 'aiEdit': 'Edición con IA',
        'currentSchema': 'Esquema actual', 'childrenCount': 'hijos',
        'aiGenerateInstruction': 'Describe la página que deseas crear',
        'editInstruction': 'Describe lo que deseas cambiar',
        'aiEditPlaceholder': 'Describe los cambios, ej: agregar un campo de búsqueda, cambiar a diseño de dos columnas...',
        'schemaPreview': 'Vista previa del esquema', 'jsonEdit': 'Edición JSON',
    },
    'fr.json': {
        'editPage': 'Modifier la page', 'visualEdit': 'Édition visuelle', 'aiEdit': 'Édition IA',
        'currentSchema': 'Schéma actuel', 'childrenCount': 'enfants',
        'aiGenerateInstruction': 'Décrivez la page que vous souhaitez créer',
        'editInstruction': 'Décrivez ce que vous voulez modifier',
        'aiEditPlaceholder': 'Décrivez les modifications, ex: ajouter une barre de recherche, passer en deux colonnes...',
        'schemaPreview': 'Aperçu du schéma', 'jsonEdit': 'Édition JSON',
    },
    'hi.json': {
        'editPage': 'पेज संपादित करें', 'visualEdit': 'दृश्य संपादन', 'aiEdit': 'AI संपादन',
        'currentSchema': 'वर्तमान स्कीमा', 'childrenCount': 'चाइल्ड',
        'aiGenerateInstruction': 'आप जो पेज बनाना चाहते हैं उसका वर्णन करें',
        'editInstruction': 'आप क्या बदलना चाहते हैं इसका वर्णन करें',
        'aiEditPlaceholder': 'बदलावों का वर्णन करें, जैसे: सर्च बॉक्स जोड़ें, दो-कॉलम लेआउट में बदलें...',
        'schemaPreview': 'स्कीमा पूर्वावलोकन', 'jsonEdit': 'JSON संपादन',
    },
    'ja.json': {
        'editPage': 'ページ編集', 'visualEdit': 'ビジュアル編集', 'aiEdit': 'AI編集',
        'currentSchema': '現在のスキーマ', 'childrenCount': '子要素',
        'aiGenerateInstruction': '作成したいページを説明してください',
        'editInstruction': '変更内容を説明してください',
        'aiEditPlaceholder': '変更内容を説明してください。例：検索ボックスを追加、2カラムレイアウトに変更...',
        'schemaPreview': 'スキーマプレビュー', 'jsonEdit': 'JSON編集',
    },
    'ko.json': {
        'editPage': '페이지 편집', 'visualEdit': '시각적 편집', 'aiEdit': 'AI 편집',
        'currentSchema': '현재 스키마', 'childrenCount': '자식',
        'aiGenerateInstruction': '생성하려는 페이지를 설명하세요',
        'editInstruction': '변경하려는 내용을 설명하세요',
        'aiEditPlaceholder': '변경 사항 설명, 예: 검색 상자 추가, 2열 레이아웃으로 변경...',
        'schemaPreview': '스키마 미리보기', 'jsonEdit': 'JSON 편집',
    },
    'ru.json': {
        'editPage': 'Редактировать страницу', 'visualEdit': 'Визуальное редактирование', 'aiEdit': 'AI-редактирование',
        'currentSchema': 'Текущая схема', 'childrenCount': 'дочерних',
        'aiGenerateInstruction': 'Опишите страницу, которую хотите создать',
        'editInstruction': 'Опишите, что хотите изменить',
        'aiEditPlaceholder': 'Опишите изменения, например: добавить поиск, изменить на двухколоночный макет...',
        'schemaPreview': 'Предпросмотр схемы', 'jsonEdit': 'JSON-редактор',
    },
    'zh-TW.json': {
        'editPage': '編輯頁面', 'visualEdit': '可視化編輯', 'aiEdit': 'AI 編輯',
        'currentSchema': '當前 Schema', 'childrenCount': '個子節點',
        'aiGenerateInstruction': '描述你想要創建的頁面功能',
        'editInstruction': '描述你想要修改的內容',
        'aiEditPlaceholder': '描述修改內容，例如：添加一個搜索框、改為兩列佈局...',
        'schemaPreview': 'Schema 預覽', 'jsonEdit': 'JSON 編輯',
    },
}

zh_cn = {
    'editPage': '编辑页面', 'visualEdit': '可视化编辑', 'aiEdit': 'AI 编辑',
    'currentSchema': '当前 Schema', 'childrenCount': '个子节点',
    'aiGenerateInstruction': '描述你想要创建的页面功能',
    'editInstruction': '描述你想要修改的内容',
    'aiEditPlaceholder': '描述修改内容，例如：添加一个搜索框、改为两列布局...',
    'schemaPreview': 'Schema 预览', 'jsonEdit': 'JSON 编辑',
}

import os
for fname in sorted(os.listdir(base)):
    if not fname.endswith('.json'):
        continue
    with open(f'{base}/{fname}', 'r', encoding='utf-8') as f:
        data = json.load(f)

    if 'settings' not in data:
        data['settings'] = {}
    if 'dynamicPages' not in data['settings']:
        data['settings']['dynamicPages'] = {}

    dp = data['settings']['dynamicPages']
    to_add = {}

    if fname == 'zh-CN.json':
        to_add.update(zh_cn)
    elif fname in translations:
        to_add.update(translations[fname])

    for k, v in to_add.items():
        if k not in dp:
            dp[k] = v

    with open(f'{base}/{fname}', 'w', encoding='utf-8') as f:
        json.dump(data, f, ensure_ascii=False, indent=2)
        f.write('\n')

    added = [k for k in to_add if k in dp]
    print(f'OK {fname}: +{len(added)} keys')

print('Done!')
