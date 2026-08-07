import { icons } from "./icons.ts";

export interface UiLogger {
  warn: (message: string) => void;
}

export interface RegisterUiHelpersOptions {
  logger?: UiLogger;
}

interface HandlebarsLike {
  registerHelper: (name: string, fn: (...args: any[]) => any) => void;
  SafeString: new (value: string) => unknown;
}

export const registerUiHelpers = (
  handlebars: HandlebarsLike,
  options: RegisterUiHelpersOptions = {},
): void => {
  const logger = options.logger ?? console;

  handlebars.registerHelper("eq", (a: unknown, b: unknown) => a == b);

  handlebars.registerHelper("concat", (...args: unknown[]) =>
    args.slice(0, -1).join(""),
  );

  handlebars.registerHelper("emptyCtx", () => ({}));

  handlebars.registerHelper(
    "icon",
    (name: string, hbOptions?: { hash?: { size?: number | string; class?: string } }) => {
      const svg = icons[name];
      if (!svg) {
        logger.warn(`Unknown icon: ${name}`);
        return "";
      }

      const hash = hbOptions?.hash ?? {};
      const size = Number(hash.size) > 0 ? Number(hash.size) : 18;
      const extraClass =
        typeof hash.class === "string" && /^[a-zA-Z0-9 _-]+$/.test(hash.class)
          ? ` ${hash.class}`
          : "";

      const out = svg
        .trim()
        .replace(/<svg([^>]*)>/, (_match, attrs: string) => {
          const cleaned = attrs.replace(/\s(width|height)="[^"]*"/g, "");
          return `<svg${cleaned} class="icon${extraClass}" width="${size}" height="${size}">`;
        });

      return new handlebars.SafeString(out);
    },
  );
};
