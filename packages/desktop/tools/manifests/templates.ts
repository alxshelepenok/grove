// @ts-nocheck
import { join } from "node:path";

import partialAccordion from "../views/partials/accordion.hbs" with { type: "text" };
import partialAlertModal from "../views/partials/alert-modal.hbs" with { type: "text" };
import partialAlert from "../views/partials/alert.hbs" with { type: "text" };
import partialBadge from "../views/partials/badge.hbs" with { type: "text" };
import partialButton from "../views/partials/button.hbs" with { type: "text" };
import partialCard from "../views/partials/card.hbs" with { type: "text" };
import partialErrorPage from "../views/partials/error-page.hbs" with { type: "text" };
import partialInput from "../views/partials/input.hbs" with { type: "text" };
import partialModal from "../views/partials/modal.hbs" with { type: "text" };
import partialNavItem from "../views/partials/nav-item.hbs" with { type: "text" };
import partialPanelFooter from "../views/partials/panel-footer.hbs" with { type: "text" };
import partialPanel from "../views/partials/panel.hbs" with { type: "text" };
import partialResumeBanner from "../views/partials/resume-banner.hbs" with { type: "text" };
import partialScrollArea from "../views/partials/scroll-area.hbs" with { type: "text" };
import partialSearchableSelect from "../views/partials/searchable-select.hbs" with { type: "text" };
import partialSegmentedBar from "../views/partials/segmented-bar.hbs" with { type: "text" };
import partialStatusModal from "../views/partials/status-modal.hbs" with { type: "text" };
import partialTabPanel from "../views/partials/tab-panel.hbs" with { type: "text" };
import partialTab from "../views/partials/tab.hbs" with { type: "text" };
import partialTabs from "../views/partials/tabs.hbs" with { type: "text" };
import partialTooltip from "../views/partials/tooltip.hbs" with { type: "text" };

export const SHARED_PARTIALS_DIR = join(import.meta.dir, "..", "views", "partials");

export const sharedPartials: Record<string, string> = {
  accordion: partialAccordion,
  "alert-modal": partialAlertModal,
  alert: partialAlert,
  badge: partialBadge,
  button: partialButton,
  card: partialCard,
  "error-page": partialErrorPage,
  input: partialInput,
  modal: partialModal,
  "nav-item": partialNavItem,
  "panel-footer": partialPanelFooter,
  panel: partialPanel,
  "resume-banner": partialResumeBanner,
  "scroll-area": partialScrollArea,
  "searchable-select": partialSearchableSelect,
  "segmented-bar": partialSegmentedBar,
  "status-modal": partialStatusModal,
  "tab-panel": partialTabPanel,
  tab: partialTab,
  tabs: partialTabs,
  tooltip: partialTooltip,
};
