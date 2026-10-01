import { readdir, readFile } from "node:fs/promises";
import { dirname, extname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const palette = join(root, "src", "color-scheme.css");
const namedColors = new Set(
  `aliceblue antiquewhite aqua aquamarine azure beige bisque black blanchedalmond blue blueviolet brown burlywood cadetblue chartreuse chocolate coral cornflowerblue cornsilk crimson cyan darkblue darkcyan darkgoldenrod darkgray darkgrey darkgreen darkkhaki darkmagenta darkolivegreen darkorange darkorchid darkred darksalmon darkseagreen darkslateblue darkslategray darkslategrey darkturquoise darkviolet deeppink deepskyblue dimgray dimgrey dodgerblue firebrick floralwhite forestgreen fuchsia gainsboro ghostwhite gold goldenrod gray grey green greenyellow honeydew hotpink indianred indigo ivory khaki lavender lavenderblush lawngreen lemonchiffon lightblue lightcoral lightcyan lightgoldenrodyellow lightgray lightgrey lightgreen lightpink lightsalmon lightseagreen lightskyblue lightslategray lightslategrey lightsteelblue lightyellow lime limegreen linen magenta maroon mediumaquamarine mediumblue mediumorchid mediumpurple mediumseagreen mediumslateblue mediumspringgreen mediumturquoise mediumvioletred midnightblue mintcream mistyrose moccasin navajowhite navy oldlace olive olivedrab orange orangered orchid palegoldenrod palegreen paleturquoise palevioletred papayawhip peachpuff peru pink plum powderblue purple rebeccapurple red rosybrown royalblue saddlebrown salmon sandybrown seagreen seashell sienna silver skyblue slateblue slategray slategrey snow springgreen steelblue tan teal thistle tomato turquoise violet wheat white whitesmoke yellow yellowgreen transparent`.split(
    " ",
  ),
);
const violations = [];

for (const file of [...(await sourceFiles(join(root, "src"))), join(root, "index.html")]) {
  if (file === palette) continue;

  const source = await readFile(file, "utf8");
  // Preserve offsets so diagnostics point to the original source lines.
  const code = source.replace(/\/\*[\s\S]*?\*\/|<!--[^]*?-->/g, (comment) =>
    comment.replace(/[^\n]/g, " "),
  );
  const literals = /#[\da-f]{3,8}\b|\b(?:rgba?|hsla?|hwb|lab|lch|oklab|oklch|color)\s*\(/gi;
  for (const match of code.matchAll(literals)) report(file, source, match.index, match[0]);

  reportNamedColors(file, source, code);
}

if (violations.length > 0) {
  console.error("Define literal colors in src/color-scheme.css and use var(--color-…) elsewhere.");
  console.error(violations.join("\n"));
  process.exitCode = 1;
} else {
  console.log("Color tokens: all frontend colors use the shared palette.");
}

async function sourceFiles(directory) {
  const files = [];

  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);

    if (entry.isDirectory()) {
      files.push(...(await sourceFiles(path)));
      continue;
    }

    if (/\.(?:css|tsx?|jsx?|svg|html)$/.test(entry.name) && !/\.(?:test|spec)\./.test(entry.name)) {
      files.push(path);
    }
  }

  return files;
}

function reportNamedColors(file, source, code) {
  const values =
    extname(file) === ".css"
      ? /(?:^|[;{])\s*[\w-]+\s*:\s*([^;{}]+)/g
      : /(?:\b(?:color|bgColor|fgColor|fill|stroke|background|backgroundColor|borderColor|outlineColor)\s*(?:=|:)\s*(?:\{\s*)?["'])([^"']+)["']/g;

  for (const value of code.matchAll(values)) {
    for (const word of value[1].matchAll(/(?<![\w-])[a-z]+(?![\w-])/gi)) {
      if (!namedColors.has(word[0].toLowerCase())) continue;

      report(file, source, value.index + value[0].indexOf(value[1]) + word.index, word[0]);
    }
  }
}

function report(file, source, index, value) {
  const line = source.slice(0, index).split("\n").length;

  violations.push(`${relative(root, file)}:${line}: literal color ${value}`);
}
