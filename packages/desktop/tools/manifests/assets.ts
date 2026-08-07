// @ts-nocheck
import { join } from "node:path";

import cssDesignSystem from "../public/css/design-system.css" with { type: "text" };
import cssFonts from "../public/css/fonts.css" with { type: "text" };
import cssBase from "../public/css/base.css" with { type: "text" };
import cssUtilities from "../public/css/utilities.css" with { type: "text" };
import cssErrors from "../public/css/errors.css" with { type: "text" };
import cssGlobal from "../public/css/global.css" with { type: "text" };

import cssCompAccordion from "../public/css/components/accordion.css" with { type: "text" };
import cssCompAlertModal from "../public/css/components/alert-modal.css" with { type: "text" };
import cssCompAlert from "../public/css/components/alert.css" with { type: "text" };
import cssCompBadge from "../public/css/components/badge.css" with { type: "text" };
import cssCompButtons from "../public/css/components/buttons.css" with { type: "text" };
import cssCompDisabled from "../public/css/components/disabled.css" with { type: "text" };
import cssCompFieldBorders from "../public/css/components/field-borders.css" with { type: "text" };
import cssCompFieldRows from "../public/css/components/field-rows.css" with { type: "text" };
import cssCompFieldset from "../public/css/components/fieldset.css" with { type: "text" };
import cssCompIcons from "../public/css/components/icons.css" with { type: "text" };
import cssCompInput from "../public/css/components/input.css" with { type: "text" };
import cssCompInputs from "../public/css/components/inputs.css" with { type: "text" };
import cssCompLabels from "../public/css/components/labels.css" with { type: "text" };
import cssCompMisc from "../public/css/components/misc.css" with { type: "text" };
import cssCompModal from "../public/css/components/modal.css" with { type: "text" };
import cssCompPanel from "../public/css/components/panel.css" with { type: "text" };
import cssCompProgress from "../public/css/components/progress.css" with { type: "text" };
import cssCompRadioCheckbox from "../public/css/components/radio-checkbox.css" with { type: "text" };
import cssCompRange from "../public/css/components/range.css" with { type: "text" };
import cssCompScrollArea from "../public/css/components/scroll-area.css" with { type: "text" };
import cssCompSearchableSelect from "../public/css/components/searchable-select.css" with { type: "text" };
import cssCompSegmentedBar from "../public/css/components/segmented-bar.css" with { type: "text" };
import cssCompSelect from "../public/css/components/select.css" with { type: "text" };
import cssCompSideRail from "../public/css/components/side-rail.css" with { type: "text" };
import cssCompStatusBar from "../public/css/components/status-bar.css" with { type: "text" };
import cssCompTables from "../public/css/components/tables.css" with { type: "text" };
import cssCompTabs from "../public/css/components/tabs.css" with { type: "text" };
import cssCompTerminal from "../public/css/components/terminal.css" with { type: "text" };
import cssCompTooltip from "../public/css/components/tooltip.css" with { type: "text" };
import cssCompTreeView from "../public/css/components/tree-view.css" with { type: "text" };
import cssCompWindow from "../public/css/components/window.css" with { type: "text" };

import jsUtilCommon from "../public/js/utils/common.js" with { type: "text" };
import jsUtilTabs from "../public/js/utils/tabs.js" with { type: "text" };
import jsUtilSearchableSelect from "../public/js/utils/searchable-select.js" with { type: "text" };
import jsUtilSegmentedBar from "../public/js/utils/segmented-bar.js" with { type: "text" };
import jsUtilDebounce from "../public/js/utils/debounce.js" with { type: "text" };
import jsUtilScrollDispatcher from "../public/js/utils/scroll-dispatcher.js" with { type: "text" };
import jsUtilAlertModal from "../public/js/utils/alert-modal.js" with { type: "text" };
import jsUtilFormatDuration from "../public/js/utils/format-duration.js" with { type: "text" };

import jsLibRxjsImports from "../public/js/lib/rxjs-imports.js" with { type: "text" };

import jsVendorD3 from "../public/js/vendor/d3.js" with { type: "text" };
import jsVendorHtmx from "../public/js/vendor/htmx.js" with { type: "text" };
import jsVendorRx from "../public/js/vendor/rx.js" with { type: "text" };

import jsIntegrationTauriBridge from "../public/js/integration/tauri-bridge.js" with { type: "text" };
import jsIntegrationWindowChrome from "../public/js/integration/window-chrome.js" with { type: "text" };

import fontGeist from "../public/fonts/Geist/Geist[wght].ttf" with { type: "file" };
import fontGeistItalic from "../public/fonts/Geist/Geist-Italic[wght].ttf" with { type: "file" };
import fontGeistMono from "../public/fonts/GeistMono/GeistMono[wght].ttf" with { type: "file" };
import fontGeistMonoItalic from "../public/fonts/GeistMono/GeistMono-Italic[wght].ttf" with { type: "file" };
import fontGeistPixel from "../public/fonts/GeistPixel/GeistPixel[ELSH].ttf" with { type: "file" };

export interface SharedTextAsset {
  path: string;
  content: string;
}

export interface SharedBinaryAsset {
  path: string;
  embeddedPath: string;
}

export const SHARED_PUBLIC_DIR = join(import.meta.dir, "..", "public");

export const sharedTextAssets: SharedTextAsset[] = [
  { path: "css/design-system.css", content: cssDesignSystem },
  { path: "css/fonts.css", content: cssFonts },
  { path: "css/base.css", content: cssBase },
  { path: "css/utilities.css", content: cssUtilities },
  { path: "css/errors.css", content: cssErrors },
  { path: "css/global.css", content: cssGlobal },
  { path: "css/components/accordion.css", content: cssCompAccordion },
  { path: "css/components/alert-modal.css", content: cssCompAlertModal },
  { path: "css/components/alert.css", content: cssCompAlert },
  { path: "css/components/badge.css", content: cssCompBadge },
  { path: "css/components/buttons.css", content: cssCompButtons },
  { path: "css/components/disabled.css", content: cssCompDisabled },
  { path: "css/components/field-borders.css", content: cssCompFieldBorders },
  { path: "css/components/field-rows.css", content: cssCompFieldRows },
  { path: "css/components/fieldset.css", content: cssCompFieldset },
  { path: "css/components/icons.css", content: cssCompIcons },
  { path: "css/components/input.css", content: cssCompInput },
  { path: "css/components/inputs.css", content: cssCompInputs },
  { path: "css/components/labels.css", content: cssCompLabels },
  { path: "css/components/misc.css", content: cssCompMisc },
  { path: "css/components/modal.css", content: cssCompModal },
  { path: "css/components/panel.css", content: cssCompPanel },
  { path: "css/components/progress.css", content: cssCompProgress },
  { path: "css/components/radio-checkbox.css", content: cssCompRadioCheckbox },
  { path: "css/components/range.css", content: cssCompRange },
  { path: "css/components/scroll-area.css", content: cssCompScrollArea },
  { path: "css/components/searchable-select.css", content: cssCompSearchableSelect },
  { path: "css/components/segmented-bar.css", content: cssCompSegmentedBar },
  { path: "css/components/select.css", content: cssCompSelect },
  { path: "css/components/side-rail.css", content: cssCompSideRail },
  { path: "css/components/status-bar.css", content: cssCompStatusBar },
  { path: "css/components/tables.css", content: cssCompTables },
  { path: "css/components/tabs.css", content: cssCompTabs },
  { path: "css/components/terminal.css", content: cssCompTerminal },
  { path: "css/components/tooltip.css", content: cssCompTooltip },
  { path: "css/components/tree-view.css", content: cssCompTreeView },
  { path: "css/components/window.css", content: cssCompWindow },
  { path: "js/utils/common.js", content: jsUtilCommon },
  { path: "js/utils/tabs.js", content: jsUtilTabs },
  { path: "js/utils/searchable-select.js", content: jsUtilSearchableSelect },
  { path: "js/utils/segmented-bar.js", content: jsUtilSegmentedBar },
  { path: "js/utils/debounce.js", content: jsUtilDebounce },
  { path: "js/utils/scroll-dispatcher.js", content: jsUtilScrollDispatcher },
  { path: "js/utils/alert-modal.js", content: jsUtilAlertModal },
  { path: "js/utils/format-duration.js", content: jsUtilFormatDuration },
  { path: "js/lib/rxjs-imports.js", content: jsLibRxjsImports },
  { path: "js/vendor/d3.js", content: jsVendorD3 },
  { path: "js/vendor/htmx.js", content: jsVendorHtmx },
  { path: "js/vendor/rx.js", content: jsVendorRx },
  { path: "js/integration/tauri-bridge.js", content: jsIntegrationTauriBridge },
  { path: "js/integration/window-chrome.js", content: jsIntegrationWindowChrome },
];

export const sharedBinaryAssets: SharedBinaryAsset[] = [
  { path: "fonts/Geist/Geist[wght].ttf", embeddedPath: fontGeist },
  { path: "fonts/Geist/Geist-Italic[wght].ttf", embeddedPath: fontGeistItalic },
  { path: "fonts/GeistMono/GeistMono[wght].ttf", embeddedPath: fontGeistMono },
  { path: "fonts/GeistMono/GeistMono-Italic[wght].ttf", embeddedPath: fontGeistMonoItalic },
  { path: "fonts/GeistPixel/GeistPixel[ELSH].ttf", embeddedPath: fontGeistPixel },
];
