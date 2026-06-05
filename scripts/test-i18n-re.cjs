const tests = [
  'something.t("foo.bar")',
  "t('foo.bar')",
  't("a.b.c")',
  't("a.b.c", { x: 1 })',
  'i18next.t("a.b.c")',
  "i18n.t('a.b.c')",
  '<Trans i18nKey="a.b.c">',
  'const { t } = useTranslation(); t("a.b.c");',
  'i18nKey="a.b.c"',
];

const T_QUOTED = /(?<![\w$.])t\s*\(\s*(['"])([^'"\n]{2,}?)\1\s*(?:[,)])/g;
const T_BACKTICK = /(?<![\w$.])t\s*\(\s*`([^`\n]{2,}?)`\s*(?:[,)])/g;
const I18NEXT_T = /\b(?:i18next|i18n|translation)\s*\.\s*t\s*\(\s*(['"])([^'"\n]{2,}?)\1\s*(?:[,)])/g;
const I18NKEY = /i18nKey\s*=\s*(['"])([^'"\n]{2,}?)\1/g;

for (const s of tests) {
  console.log("---", s);
  for (const re of [T_QUOTED, T_BACKTICK, I18NEXT_T, I18NKEY]) {
    re.lastIndex = 0;
    let m;
    while ((m = re.exec(s)) !== null) {
      console.log("  match:", m[0], "->", JSON.stringify(m[1]));
    }
  }
}
