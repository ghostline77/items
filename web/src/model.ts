import type { BrowserJson, BrowserMedia } from "@ghostline/ui-core";
export interface Entry {
  id: string;
  label?: string;
  native_name?: string;
  native_id?: string;
  category?: string;
  kind?: string;
  control?: string;
  native?: { name?: string; ui_slot?: string };
  roles?: string[];
  resource?: { Path?: string; Hash?: string };
  selection?: string;
  selections?: string[];
  preview_state?: string;
  media?: BrowserMedia[];
  appearance?: string;
  detail_script?: string;
  parts?: string[];
  assemblies?: string[];
  choices?: string[];
  diagnostics?: string[];
  source?: BrowserJson;
  context?: {
    prop_components?: {
      part: string;
      component: string;
      native_world_transform?: number[];
    }[];
  };
}
export interface Catalogue {
  items: Entry[];
  props?: Entry[];
  controls: Entry[];
  choices: Entry[];
  assets: Record<string, Entry>;
  selections: Record<string, Entry>;
  game_version?: string;
}
declare global {
  var GHOSTLINE_CATALOGUE: Catalogue | undefined;
  var GHOSTLINE_MEDIA_BASE: string | undefined;
  var GHOSTLINE_DETAILS: Record<string, Entry> | undefined;
}
export const resourceText = (r: Entry["resource"]): string =>
  r?.Path ?? r?.Hash ?? "";
export const label = (v: Entry): string =>
  v.label ?? v.native?.name ?? (resourceText(v.resource) || v.id);
export const technical = (v: Entry): string =>
  v.native_name ?? (resourceText(v.resource) || v.native_id || v.id);
export const category = (v: Entry, controls: Map<string, Entry>): string =>
  v.category ??
  v.native?.ui_slot ??
  controls.get(v.control || "")?.native?.ui_slot ??
  v.roles?.[0] ??
  "Unclassified";
export const selectionIds = (v: Entry, data: Catalogue): string[] =>
  v.selections ??
  (v.selection
    ? [v.selection]
    : Object.hasOwn(data.selections, v.id)
      ? [v.id]
      : []);
export const previews = (v: Entry, data: Catalogue): BrowserMedia[] =>
  selectionIds(v, data).flatMap((id) =>
    (data.selections[id]?.media ?? []).map((media) => ({
      ...media,
      appearance: data.selections[id]?.appearance || null,
    })),
  );
export function groups(data: Catalogue): Record<string, Entry[]> {
  return {
    items: [...data.items].sort(
      (a, b) =>
        Number(a.label === a.native_name) - Number(b.label === b.native_name) ||
        label(a).localeCompare(label(b)),
    ),
    clothing: Object.values(data.assets).filter((v) =>
      v.roles?.includes("clothing"),
    ),
    props: [...(data.props || [])].sort(
      (a, b) =>
        Number(a.kind !== "assembly") - Number(b.kind !== "assembly") ||
        label(a).localeCompare(label(b)),
    ),
    controls: data.controls,
    choices: data.choices,
    assets: Object.values(data.assets),
  };
}
export function filterEntries(
  entries: Entry[],
  data: Catalogue,
  controls: Map<string, Entry>,
  query: string,
  cat: string,
  rendered: boolean,
  representation: string,
): Entry[] {
  const terms = query.toLowerCase().trim().split(/\s+/).filter(Boolean);
  return entries.filter(
    (v) =>
      (!representation || v.kind === representation) &&
      (!cat || category(v, controls) === cat) &&
      (!rendered || previews(v, data).length > 0) &&
      terms.every((term) =>
        JSON.stringify([
          v,
          category(v, controls),
          ...(v.control
            ? selectionIds(v, data).map((id) => data.selections[id])
            : []),
        ])
          .toLowerCase()
          .includes(term),
      ),
  );
}
