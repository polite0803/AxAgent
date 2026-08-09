# -*- coding: utf-8 -*-
"""给 9 种语言补 chat.workflowRunner.*（文本级插入，避免 json.dump 全量重排大 diff）"""
import re
import sys

translations = {
    "zh-TW": {
        "title": "股票工作流執行中",
        "inputTheme": "輸入主題（可選）",
        "themeHint": "輸入感興趣的主題關鍵詞，工作流將圍繞這些主題篩選候選股票。可留空運行全市場篩選。",
        "themePlaceholder": "如：AI 晶片、新能源、CXO",
        "sourceUser": "主題",
        "run": "執行",
        "rerun": "重新執行",
        "cancel": "取消",
        "progress": "執行進度",
        "completed": "執行完成",
        "close": "關閉",
        "noResult": "未找到符合條件的股票",
    },
    "ja": {
        "title": "株式ワークフロー実行中",
        "inputTheme": "テーマを入力（任意）",
        "themeHint": "興味のあるテーマのキーワードを入力してください。ワークフローはこれらのテーマに沿って候補銘柄を絞り込みます。空欄で全市場スキャン。",
        "themePlaceholder": "例：AIチップ、新エネルギー、CXO",
        "sourceUser": "テーマ",
        "run": "実行",
        "rerun": "再実行",
        "cancel": "キャンセル",
        "progress": "実行進捗",
        "completed": "実行完了",
        "close": "閉じる",
        "noResult": "条件に一致する銘柄が見つかりません",
    },
    "ko": {
        "title": "주식 워크플로 실행 중",
        "inputTheme": "테마 입력 (선택)",
        "themeHint": "관심 있는 테마 키워드를 입력하세요. 워크플로가 이 테마에 따라 후보 주식을 선별합니다. 비워두면 전체 시장 스캔.",
        "themePlaceholder": "예: AI 칩, 신재생에너지, CXO",
        "sourceUser": "테마",
        "run": "실행",
        "rerun": "다시 실행",
        "cancel": "취소",
        "progress": "실행 진행률",
        "completed": "실행 완료",
        "close": "닫기",
        "noResult": "조건에 맞는 주식을 찾지 못했습니다",
    },
    "de": {
        "title": "Aktien-Workflow läuft",
        "inputTheme": "Thema eingeben (optional)",
        "themeHint": "Themen-Keywords eingeben – der Workflow filtert Kandidaten rund um diese Themen. Leer lassen für Full-Market-Screening.",
        "themePlaceholder": "z. B.: AI-Chips, erneuerbare Energien, CXO",
        "sourceUser": "Thema",
        "run": "Ausführen",
        "rerun": "Erneut ausführen",
        "cancel": "Abbrechen",
        "progress": "Fortschritt",
        "completed": "Abgeschlossen",
        "close": "Schließen",
        "noResult": "Keine passenden Aktien gefunden",
    },
    "fr": {
        "title": "Workflow actions en cours",
        "inputTheme": "Entrer un thème (facultatif)",
        "themeHint": "Saisissez des mots-clés de thème – le workflow sélectionnera les actions candidates autour de ces thèmes. Laisser vide pour un scan complet du marché.",
        "themePlaceholder": "ex. : puces IA, énergies nouvelles, CXO",
        "sourceUser": "Thème",
        "run": "Exécuter",
        "rerun": "Réexécuter",
        "cancel": "Annuler",
        "progress": "Progression",
        "completed": "Terminé",
        "close": "Fermer",
        "noResult": "Aucune action correspondante trouvée",
    },
    "es": {
        "title": "Flujo de trabajo de acciones en ejecución",
        "inputTheme": "Ingresar tema (opcional)",
        "themeHint": "Ingrese palabras clave de tema: el flujo de trabajo seleccionará acciones candidatas en torno a estos temas. Deje vacío para un escaneo de todo el mercado.",
        "themePlaceholder": "p. ej.: chips IA, energías renovables, CXO",
        "sourceUser": "Tema",
        "run": "Ejecutar",
        "rerun": "Volver a ejecutar",
        "cancel": "Cancelar",
        "progress": "Progreso",
        "completed": "Completado",
        "close": "Cerrar",
        "noResult": "No se encontraron acciones que coincidan",
    },
    "ru": {
        "title": "Выполнение фондового рабочего процесса",
        "inputTheme": "Введите тему (необязательно)",
        "themeHint": "Введите ключевые слова темы — рабочий процесс отберёт акции-кандидаты по этим темам. Оставьте пустым для сканирования всего рынка.",
        "themePlaceholder": "напр.: чипы ИИ, новые источники энергии, CXO",
        "sourceUser": "Тема",
        "run": "Запустить",
        "rerun": "Перезапустить",
        "cancel": "Отмена",
        "progress": "Ход выполнения",
        "completed": "Завершено",
        "close": "Закрыть",
        "noResult": "Подходящие акции не найдены",
    },
    "hi": {
        "title": "स्टॉक वर्कफ़्लो चल रहा है",
        "inputTheme": "विषय दर्ज करें (वैकल्पिक)",
        "themeHint": "रुचि के विषय कीवर्ड दर्ज करें — वर्कफ़्लो इन विषयों के आसपास उम्मीदवार स्टॉक फ़िल्टर करेगा। पूरे बाज़ार स्कैन के लिए खाली छोड़ें।",
        "themePlaceholder": "जैसे: AI चिप्स, नई ऊर्जा, CXO",
        "sourceUser": "विषय",
        "run": "चलाएँ",
        "rerun": "फिर चलाएँ",
        "cancel": "रद्द करें",
        "progress": "प्रगति",
        "completed": "पूर्ण",
        "close": "बंद करें",
        "noResult": "कोई मेल खाता स्टॉक नहीं मिला",
    },
    "ar": {
        "title": "تنفيذ سير عمل الأسهم",
        "inputTheme": "أدخل الموضوع (اختياري)",
        "themeHint": "أدخل كلمات مفتاحية للموضوع — سيقوم سير العمل بتصفية الأسهم المرشحة حول هذه المواضيع. اتركه فارغًا لمسح السوق بالكامل.",
        "themePlaceholder": "مثال: رقائق الذكاء الاصطناعي، الطاقة الجديدة، CXO",
        "sourceUser": "الموضوع",
        "run": "تشغيل",
        "rerun": "إعادة تشغيل",
        "cancel": "إلغاء",
        "progress": "التقدم",
        "completed": "اكتمل",
        "close": "إغلاق",
        "noResult": "لم يتم العثور على أسهم مطابقة",
    },
}

def detect_newline(path):
    with open(path, "rb") as f:
        raw = f.read()
    return "\r\n" if b"\r\n" in raw else "\n"

def main():
    for lang, wr in translations.items():
        path = f"src/i18n/locales/{lang}.json"
        nl = detect_newline(path)
        with open(path, "r", encoding="utf-8", newline="") as f:
            text = f.read()

        if '"workflowRunner"' in text:
            print(f"{lang}: already exists, skip")
            continue

        # 构造 workflowRunner 块（chat 内 key 缩进 4 空格，内部 6 空格）
        items = list(wr.items())
        lines = [f'    "workflowRunner": {{']
        for i, (k, v) in enumerate(items):
            comma = "," if i < len(items) - 1 else ""
            lines.append(f'      "{k}": "{v}"{comma}')
        lines.append("    },")
        block = nl.join(lines) + nl

        m = re.search(r"(\n\s*\"chat\": \{\n)", text)
        if not m:
            print(f"{lang}: 'chat' key not found, abort")
            continue
        pos = m.end()
        text = text[:pos] + block + text[pos:]

        with open(path, "w", encoding="utf-8", newline="") as f:
            f.write(text)
        print(f"{lang}: inserted workflowRunner ({len(items)} keys)")

if __name__ == "__main__":
    main()
