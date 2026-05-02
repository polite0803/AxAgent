import fs from "fs";

const zhcn = JSON.parse(fs.readFileSync("src/i18n/locales/zh-CN.json", "utf8"));
console.log("zhcn.settings.theme:", zhcn.settings.theme);
console.log("zhcn.settings.shortcuts:", zhcn.settings.shortcuts);
console.log("zhcn.style.dimensions:", zhcn.style.dimensions);
