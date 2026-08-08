"""批量更新所有语言文件，添加 domainWorkflows 相关翻译键。"""
import json
from pathlib import Path

BASE_DIR = Path(__file__).resolve().parent.parent / "src" / "i18n" / "locales"

# 各语言的翻译
translations = {
    "zh-CN": {
        "domainWorkflows": "领域工作流",
        "loadingDomainWorkflows": "加载领域工作流中...",
    },
    "en-US": {
        "domainWorkflows": "Domain Workflows",
        "loadingDomainWorkflows": "Loading domain workflows...",
    },
    "zh-TW": {
        "domainWorkflows": "領域工作流",
        "loadingDomainWorkflows": "載入領域工作流中...",
    },
    "ja": {
        "domainWorkflows": "ドメインワークフロー",
        "loadingDomainWorkflows": "ドメインワークフローを読み込み中...",
    },
    "ko": {
        "domainWorkflows": "도메인 워크플로우",
        "loadingDomainWorkflows": "도메인 워크플로우 로딩 중...",
    },
    "ru": {
        "domainWorkflows": "Доменные workflow",
        "loadingDomainWorkflows": "Загрузка доменных workflow...",
    },
    "fr": {
        "domainWorkflows": "Workflows de domaine",
        "loadingDomainWorkflows": "Chargement des workflows de domaine...",
    },
    "es": {
        "domainWorkflows": "Flujos de dominio",
        "loadingDomainWorkflows": "Cargando flujos de dominio...",
    },
    "de": {
        "domainWorkflows": "Domänen-Workflows",
        "loadingDomainWorkflows": "Lade Domänen-Workflows...",
    },
    "hi": {
        "domainWorkflows": "डोमेन वर्कफ़्लो",
        "loadingDomainWorkflows": "डोमेन वर्कफ़्लो लोड हो रहे हैं...",
    },
    "ar": {
        "domainWorkflows": "تدفقات المجال",
        "loadingDomainWorkflows": "جاري تحميل تدفقات المجال...",
    },
}

for lang, trans in translations.items():
    file_path = BASE_DIR / f"{lang}.json"
    if not file_path.exists():
        print(f"⚠️  文件不存在: {file_path}")
        continue

    with open(file_path, "r", encoding="utf-8") as f:
        data = json.load(f)

    # 导航到 opc.industry 部分
    if "opc" in data and "industry" in data["opc"]:
        industry = data["opc"]["industry"]
        
        # 添加新键
        industry["domainWorkflows"] = trans["domainWorkflows"]
        industry["loadingDomainWorkflows"] = trans["loadingDomainWorkflows"]
        
        # 按字母顺序重新排序键
        sorted_industry = dict(sorted(industry.items()))
        data["opc"]["industry"] = sorted_industry
        
        with open(file_path, "w", encoding="utf-8") as f:
            json.dump(data, f, ensure_ascii=False, indent=2)
            f.write("\n")
        
        print(f"✅ 已更新: {lang}")
    else:
        print(f"⚠️  未找到 opc.industry: {lang}")

print("\n✅ 完成！")
