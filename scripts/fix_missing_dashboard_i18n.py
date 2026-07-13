#!/usr/bin/env python3
"""为所有 11 个 locale 文件补全缺失的 dashboard.* key。"""
import json
import os

missing_keys = [
    'usageTrend', 'noUsageData', 'costOverview', 'totalCost',
    'avgCostPerSession', 'totalAgentTokens', 'dailyAvgCost',
    'costByProvider', 'inputTokens', 'outputTokens',
]

en_vals = {
    'usageTrend': 'Usage Trend',
    'noUsageData': 'No usage data',
    'costOverview': 'Cost Overview',
    'totalCost': 'Total Cost',
    'avgCostPerSession': 'Avg Cost / Session',
    'totalAgentTokens': 'Total Agent Tokens',
    'dailyAvgCost': 'Daily Avg Cost',
    'costByProvider': 'Cost by Provider',
    'inputTokens': 'Input Tokens',
    'outputTokens': 'Output Tokens',
}

zh_vals = {
    'usageTrend': '使用趋势',
    'noUsageData': '暂无使用数据',
    'costOverview': '成本概览',
    'totalCost': '总费用',
    'avgCostPerSession': '单次会话平均费用',
    'totalAgentTokens': 'Agent Token 总数',
    'dailyAvgCost': '日均费用',
    'costByProvider': '按 Provider 分摊费用',
    'inputTokens': '输入 Token',
    'outputTokens': '输出 Token',
}

zh_tw_vals = {
    'usageTrend': '使用趨勢',
    'noUsageData': '暫無使用資料',
    'costOverview': '成本總覽',
    'totalCost': '總費用',
    'avgCostPerSession': '單次會話平均費用',
    'totalAgentTokens': 'Agent Token 總數',
    'dailyAvgCost': '日均費用',
    'costByProvider': '按 Provider 分攤費用',
    'inputTokens': '輸入 Token',
    'outputTokens': '輸出 Token',
}

ja_vals = {
    'usageTrend': '利用傾向',
    'noUsageData': '利用データなし',
    'costOverview': 'コスト概要',
    'totalCost': '総費用',
    'avgCostPerSession': 'セッション平均コスト',
    'totalAgentTokens': 'Agent 総トークン',
    'dailyAvgCost': '日次平均コスト',
    'costByProvider': 'プロバイダー別コスト',
    'inputTokens': '入力トークン',
    'outputTokens': '出力トークン',
}

ko_vals = {
    'usageTrend': '사용 추이',
    'noUsageData': '사용 데이터 없음',
    'costOverview': '비용 개요',
    'totalCost': '총 비용',
    'avgCostPerSession': '세션당 평균 비용',
    'totalAgentTokens': 'Agent 총 토큰',
    'dailyAvgCost': '일평균 비용',
    'costByProvider': '공급자별 비용',
    'inputTokens': '입력 토큰',
    'outputTokens': '출력 토큰',
}

de_vals = {
    'usageTrend': 'Nutzungstrend',
    'noUsageData': 'Keine Nutzungsdaten',
    'costOverview': 'Kostenübersicht',
    'totalCost': 'Gesamtkosten',
    'avgCostPerSession': 'Ø Kosten / Sitzung',
    'totalAgentTokens': 'Agent-Gesamttokens',
    'dailyAvgCost': 'Tägl. Durchschnittskosten',
    'costByProvider': 'Kosten nach Anbieter',
    'inputTokens': 'Eingabe-Tokens',
    'outputTokens': 'Ausgabe-Tokens',
}

fr_vals = {
    'usageTrend': "Tendance d'utilisation",
    'noUsageData': "Aucune donnée d'utilisation",
    'costOverview': 'Aperçu des coûts',
    'totalCost': 'Coût total',
    'avgCostPerSession': 'Coût moyen / session',
    'totalAgentTokens': 'Tokens Agent totaux',
    'dailyAvgCost': 'Coût moyen quotidien',
    'costByProvider': 'Coût par fournisseur',
    'inputTokens': "Tokens d'entrée",
    'outputTokens': 'Tokens de sortie',
}

es_vals = {
    'usageTrend': 'Tendencia de uso',
    'noUsageData': 'Sin datos de uso',
    'costOverview': 'Resumen de costes',
    'totalCost': 'Coste total',
    'avgCostPerSession': 'Coste medio / sesión',
    'totalAgentTokens': 'Tokens totales del agente',
    'dailyAvgCost': 'Coste medio diario',
    'costByProvider': 'Coste por proveedor',
    'inputTokens': 'Tokens de entrada',
    'outputTokens': 'Tokens de salida',
}

ar_vals = {
    'usageTrend': 'اتجاه الاستخدام',
    'noUsageData': 'لا توجد بيانات استخدام',
    'costOverview': 'نظرة عامة على التكلفة',
    'totalCost': 'التكلفة الإجمالية',
    'avgCostPerSession': 'متوسط التكلفة / الجلسة',
    'totalAgentTokens': 'إجمالي رموز الوكيل',
    'dailyAvgCost': 'متوسط التكلفة اليومية',
    'costByProvider': 'التكلفة حسب الموفر',
    'inputTokens': 'رموز الإدخال',
    'outputTokens': 'رموز الإخراج',
}

ru_vals = {
    'usageTrend': 'Тенденция использования',
    'noUsageData': 'Нет данных об использовании',
    'costOverview': 'Обзор затрат',
    'totalCost': 'Общая стоимость',
    'avgCostPerSession': 'Средняя стоимость / сессия',
    'totalAgentTokens': 'Всего токенов агента',
    'dailyAvgCost': 'Среднедневная стоимость',
    'costByProvider': 'Стоимость по провайдеру',
    'inputTokens': 'Входные токены',
    'outputTokens': 'Выходные токены',
}

hi_vals = {
    'usageTrend': 'उपयोग प्रवृत्ति',
    'noUsageData': 'कोई उपयोग डेटा नहीं',
    'costOverview': 'लागत अवलोकन',
    'totalCost': 'कुल लागत',
    'avgCostPerSession': 'औसत लागत / सत्र',
    'totalAgentTokens': 'कुल Agent टोकन',
    'dailyAvgCost': 'दैनिक औसत लागत',
    'costByProvider': 'प्रदाता अनुसार लागत',
    'inputTokens': 'इनपुट टोकन',
    'outputTokens': 'आउटपुट टोकन',
}

localize = {
    'zh-CN.json': zh_vals,
    'zh-TW.json': zh_tw_vals,
    'ja.json': ja_vals,
    'ko.json': ko_vals,
    'de.json': de_vals,
    'fr.json': fr_vals,
    'es.json': es_vals,
    'ar.json': ar_vals,
    'ru.json': ru_vals,
    'hi.json': hi_vals,
    'en-US.json': en_vals,
}

base = 'src/i18n/locales'
for fname, vals in localize.items():
    p = os.path.join(base, fname)
    with open(p, encoding='utf-8') as f:
        d = json.load(f)
    if 'dashboard' not in d or not isinstance(d.get('dashboard'), dict):
        d['dashboard'] = {}
    added = []
    for k, v in vals.items():
        if k not in d['dashboard']:
            d['dashboard'][k] = v
            added.append(k)
    if added:
        with open(p, 'w', encoding='utf-8') as f:
            json.dump(d, f, ensure_ascii=False, indent=2)
            f.write('\n')
        print(f'{fname}: added {len(added)} keys -> {added}')
    else:
        print(f'{fname}: no missing keys')
