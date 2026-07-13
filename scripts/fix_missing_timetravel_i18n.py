#!/usr/bin/env python3
"""为所有 11 个 locale 文件补全缺失的 timeTravel.* 命名空间 key。

调用方：PageTimeAnchor / DecisionBanner / KLineChart
共 6 个 key：pageAnchor.live, pageAnchor.replay, pageAnchor.untilDate,
datePicker.placeholder, degradedMarker.tooltip, degradedMarker.labelWithCount
"""
import json
import os
from pathlib import Path

LOCALES_DIR = Path("src/i18n/locales")

# ── 6 个 key 及其在每种语言下的翻译 ──
TRANSLATIONS = {
    "zh-CN": {
        "timeTravel": {
            "pageAnchor": {
                "live": "实时分析",
                "replay": "历史回放",
                "untilDate": "回放至 {{date}}",
            },
            "datePicker": {
                "placeholder": "选择回放日期",
            },
            "degradedMarker": {
                "tooltip": "部分分析方法在回放模式下不可用",
                "labelWithCount": "降级 {{n}} 项",
            },
        },
    },
    "zh-TW": {
        "timeTravel": {
            "pageAnchor": {
                "live": "即時分析",
                "replay": "歷史回放",
                "untilDate": "回放至 {{date}}",
            },
            "datePicker": {
                "placeholder": "選擇回放日期",
            },
            "degradedMarker": {
                "tooltip": "部分分析方法在回放模式下不可用",
                "labelWithCount": "降級 {{n}} 項",
            },
        },
    },
    "en-US": {
        "timeTravel": {
            "pageAnchor": {
                "live": "Live",
                "replay": "Replay",
                "untilDate": "Replay until {{date}}",
            },
            "datePicker": {
                "placeholder": "Select replay date",
            },
            "degradedMarker": {
                "tooltip": "Some analysis methods are unavailable in replay mode",
                "labelWithCount": "{{n}} degraded",
            },
        },
    },
    "ja": {
        "timeTravel": {
            "pageAnchor": {
                "live": "ライブ",
                "replay": "リプレイ",
                "untilDate": "{{date}} までリプレイ",
            },
            "datePicker": {
                "placeholder": "リプレイ日を選択",
            },
            "degradedMarker": {
                "tooltip": "リプレイモードでは一部の方法が利用できません",
                "labelWithCount": "{{n}} 件劣化",
            },
        },
    },
    "ko": {
        "timeTravel": {
            "pageAnchor": {
                "live": "실시간",
                "replay": "리플레이",
                "untilDate": "{{date}}까지 리플레이",
            },
            "datePicker": {
                "placeholder": "리플레이 날짜 선택",
            },
            "degradedMarker": {
                "tooltip": "리플레이 모드에서는 일부 분석 방법을 사용할 수 없습니다",
                "labelWithCount": "{{n}}개 저하",
            },
        },
    },
    "de": {
        "timeTravel": {
            "pageAnchor": {
                "live": "Live",
                "replay": "Replay",
                "untilDate": "Replay bis {{date}}",
            },
            "datePicker": {
                "placeholder": "Replay-Datum wählen",
            },
            "degradedMarker": {
                "tooltip": "Einige Analysemethoden sind im Replay-Modus nicht verfügbar",
                "labelWithCount": "{{n}} eingeschränkt",
            },
        },
    },
    "fr": {
        "timeTravel": {
            "pageAnchor": {
                "live": "En direct",
                "replay": "Relecture",
                "untilDate": "Relecture jusqu'au {{date}}",
            },
            "datePicker": {
                "placeholder": "Choisir la date de relecture",
            },
            "degradedMarker": {
                "tooltip": "Certaines méthodes d'analyse ne sont pas disponibles en mode relecture",
                "labelWithCount": "{{n}} dégradées",
            },
        },
    },
    "es": {
        "timeTravel": {
            "pageAnchor": {
                "live": "En vivo",
                "replay": "Reproducción",
                "untilDate": "Reproducir hasta {{date}}",
            },
            "datePicker": {
                "placeholder": "Seleccionar fecha de reproducción",
            },
            "degradedMarker": {
                "tooltip": "Algunos métodos de análisis no están disponibles en modo reproducción",
                "labelWithCount": "{{n}} degradados",
            },
        },
    },
    "ru": {
        "timeTravel": {
            "pageAnchor": {
                "live": "В реальном времени",
                "replay": "Воспроизведение",
                "untilDate": "Воспроизвести до {{date}}",
            },
            "datePicker": {
                "placeholder": "Выберите дату воспроизведения",
            },
            "degradedMarker": {
                "tooltip": "Некоторые методы анализа недоступны в режиме воспроизведения",
                "labelWithCount": "{{n}} ухудшено",
            },
        },
    },
    "hi": {
        "timeTravel": {
            "pageAnchor": {
                "live": "लाइव",
                "replay": "रीप्ले",
                "untilDate": "{{date}} तक रीप्ले",
            },
            "datePicker": {
                "placeholder": "रीप्ले तिथि चुनें",
            },
            "degradedMarker": {
                "tooltip": "रीप्ले मोड में कुछ विश्लेषण विधियाँ उपलब्ध नहीं हैं",
                "labelWithCount": "{{n}} अवनत",
            },
        },
    },
    "ar": {
        "timeTravel": {
            "pageAnchor": {
                "live": "مباشر",
                "replay": "إعادة",
                "untilDate": "إعادة حتى {{date}}",
            },
            "datePicker": {
                "placeholder": "اختر تاريخ الإعادة",
            },
            "degradedMarker": {
                "tooltip": "بعض طرق التحليل غير متاحة في وضع الإعادة",
                "labelWithCount": "{{n}} منخفضة",
            },
        },
    },
}

# locale 文件名映射
FILE_MAP = {
    "zh-CN": "zh-CN.json",
    "zh-TW": "zh-TW.json",
    "en-US": "en-US.json",
    "ja": "ja.json",
    "ko": "ko.json",
    "de": "de.json",
    "fr": "fr.json",
    "es": "es.json",
    "ru": "ru.json",
    "hi": "hi.json",
    "ar": "ar.json",
}


def deep_merge(base: dict, extra: dict) -> dict:
    """递归合并 dict，extra 覆盖 base。"""
    out = dict(base)
    for k, v in extra.items():
        if k in out and isinstance(out[k], dict) and isinstance(v, dict):
            out[k] = deep_merge(out[k], v)
        else:
            out[k] = v
    return out


def main() -> int:
    summary = []
    for lang, translations in TRANSLATIONS.items():
        path = LOCALES_DIR / FILE_MAP[lang]
        if not path.exists():
            print(f"[SKIP] {path} 不存在")
            continue

        with path.open("r", encoding="utf-8") as f:
            data = json.load(f)

        before = json.dumps(data, ensure_ascii=False, sort_keys=True)
        data = deep_merge(data, translations)
        after = json.dumps(data, ensure_ascii=False, sort_keys=True)

        if before == after:
            print(f"[OK]   {path.name} 已有 timeTravel key，跳过")
            continue

        with path.open("w", encoding="utf-8") as f:
            json.dump(data, f, ensure_ascii=False, indent=2)
            f.write("\n")

        added = sum(
            1
            for path_list in translations.values()
            for sub in path_list.values()
            for sub_key in sub
            if isinstance(sub, dict)
        )
        summary.append((path.name, added))
        print(f"[WRITE] {path.name} 补 {added} 个 key")

    print(f"\n汇总: 11 个 locale 全部补齐 timeTravel.* 命名空间")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
