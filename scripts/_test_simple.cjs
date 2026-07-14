const fs = require("fs");
fs.writeFileSync("scripts/_test_output.txt", "Hello from Node.js! " + new Date().toISOString(), "utf8");
console.log("Test output written");
