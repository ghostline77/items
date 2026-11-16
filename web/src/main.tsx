import { useEffect, useMemo, useState } from "react";
import { createRoot } from "react-dom/client";
import {
  AppShell,
  Button,
  CopyJson,
  Input,
  Select,
  readBrowserScript,
  useHash,
} from "@ghostline/ui-core";
import {
  category,
  filterEntries,
  groups,
  label,
  previews,
  resourceText,
  selectionIds,
  technical,
} from "./model";
import type { Catalogue, Entry } from "./model";
import "./style.css";
const entryHref = (id: string): string => "#entry/" + encodeURIComponent(id);
async function loadDetail(entry: Entry): Promise<Entry> {
  if (!entry.detail_script) return entry;
  // A published shard contains many entries. Cache its script load, then resolve
  // this identity from the shared index rather than caching the first entry.
  if (!globalThis.GHOSTLINE_DETAILS?.[entry.id])
    await readBrowserScript(entry.detail_script, () => null);
  return globalThis.GHOSTLINE_DETAILS?.[entry.id] || entry;
}
function Detail({
  entry,
  data,
  index,
  controls,
}: {
  entry: Entry | undefined;
  data: Catalogue;
  index: Map<string, Entry>;
  controls: Map<string, Entry>;
}) {
  const [loaded, setLoaded] = useState<Entry | null>(null),
    [variants, setVariants] = useState<Entry[]>([]),
    [error, setError] = useState("");
  useEffect(() => {
    let current = true;
    setLoaded(null);
    setVariants([]);
    setError("");
    if (entry)
      loadDetail(entry)
        .then(async (value) => {
          if (!current) return;
          setLoaded(value);
          document.title = `${label(value)} · Ghostline Items`;
          const selections =
            value.kind === "assembly"
              ? await Promise.all(
                  selectionIds(value, data).flatMap((id) =>
                    index.get(id) ? [loadDetail(index.get(id)!)] : [],
                  ),
                )
              : [value];
          if (current) setVariants(selections);
        })
        .catch((e) => {
          if (current) setError(String(e));
        });
    return () => {
      current = false;
    };
  }, [entry?.id]);
  if (!entry)
    return (
      <>
        <h1 className="text-3xl font-semibold">Entry not found</h1>
        <p>This identity is not present in this catalogue.</p>
      </>
    );
  if (error)
    return (
      <p role="alert" className="text-danger">
        {error}
      </p>
    );
  if (!loaded) return <p role="status">Loading native details…</p>;
  const value = loaded,
    media = previews(value, data),
    reference = {
      native_name: value.native_name,
      source: value.source,
      ...(value.control ? { locator: value.id } : {}),
      variants: selectionIds(value, data).map((id) => ({
        resource: resourceText(data.selections[id]?.resource),
        appearance: data.selections[id]?.appearance,
      })),
    };
  return (
    <div className="space-y-6">
      <p className="text-xs uppercase tracking-widest text-muted">
        {category(value, controls)}
      </p>
      <h1 className="text-3xl font-semibold sm:text-4xl">{label(value)}</h1>
      <p className="break-all text-xs text-muted">{value.id}</p>
      <CopyJson value={reference} />
      <pre className="overflow-auto whitespace-pre-wrap break-all rounded border border-line bg-panel p-4 text-xs">
        {JSON.stringify(reference, null, 2)}
      </pre>
      {media.length ? (
        <>
          <div className="grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-3">
            {media.map((m, i) => (
              <figure
                key={`${m.path}-${i}`}
                className="rounded-lg border border-line bg-panel p-3"
              >
                <img
                  className="aspect-square w-full object-contain"
                  loading="lazy"
                  src={
                    (globalThis.GHOSTLINE_MEDIA_BASE
                      ? globalThis.GHOSTLINE_MEDIA_BASE.replace(/\/$/, "") + "/"
                      : "") + m.path
                  }
                  alt={`${label(value)} — ${m.view}`}
                />
                <figcaption className="mt-2 text-xs text-muted">
                  {m.appearance ?? "native default"} / {m.view} / {m.size}px
                </figcaption>
              </figure>
            ))}
          </div>
          <p className="text-sm text-muted">
            {value.kind === "assembly"
              ? "Complete native entity in its rig rest pose, including its component placements and material bindings."
              : value.kind === "mesh"
                ? "Individual native mesh. See containing assemblies for complete objects and placement context."
                : "Isolated native resource preview. Combined character fit and game material fidelity require their stated context and verification."}
          </p>
        </>
      ) : (
        <p className="text-muted">
          No rendered preview is published for this entry. Native metadata and
          selection references remain available.
        </p>
      )}
      {(
        [
          ["Assembly parts", value.parts || []],
          ["Containing assemblies", value.assemblies || []],
          [
            "Linked selections and choices",
            [
              ...selectionIds(value, data),
              ...(value.choices || []),
              ...(value.control ? [value.control] : []),
            ],
          ],
        ] as const
      ).map(
        ([title, ids]) =>
          ids.length > 0 && (
            <section key={title} className="space-y-3">
              <h2 className="text-xl font-semibold">{title}</h2>
              <div className="flex flex-wrap gap-3">
                {ids.map((id, i) => (
                  <a
                    key={`${id}-${i}`}
                    className="rounded border border-line px-3 py-2 text-sm text-accent"
                    href={entryHref(id)}
                  >
                    {label(index.get(id) || { id })}
                  </a>
                ))}
              </div>
            </section>
          ),
      )}
      {!!value.diagnostics?.length && (
        <section className="space-y-3">
          <h2 className="text-xl font-semibold">Source and preview notes</h2>
          {value.diagnostics.map((note, i) => (
            <p key={i} className="text-sm text-warning">
              {note}
            </p>
          ))}
        </section>
      )}
      {variants
        .filter((v) => v.context?.prop_components?.length)
        .map((v) => (
          <section key={v.id} className="space-y-3">
            <h2 className="text-xl font-semibold">
              Placed components / {v.appearance ?? "native default"}
            </h2>
            <p className="text-sm text-muted">
              Entity-relative positions in metres, native Z up. Full matrices
              and rig/slot bindings are in the selection JSON.
            </p>
            <div className="overflow-auto">
              <table className="w-full text-left text-sm">
                <thead>
                  <tr>
                    {["Native component", "Individual mesh", "X / Y / Z"].map(
                      (t) => (
                        <th className="p-3" key={t}>
                          {t}
                        </th>
                      ),
                    )}
                  </tr>
                </thead>
                <tbody>
                  {v.context?.prop_components?.map((p, i) => (
                    <tr className="border-t border-line" key={i}>
                      <td className="p-3">{p.component}</td>
                      <td className="p-3">
                        <a className="text-accent" href={entryHref(p.part)}>
                          {label(index.get(p.part) || { id: p.part })}
                        </a>
                      </td>
                      <td className="p-3">
                        {p.native_world_transform
                          ?.slice(12, 15)
                          .map((n) => n.toFixed(4))
                          .join(" / ") ?? "Unresolved"}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </section>
        ))}
      <section className="space-y-3">
        <h2 className="text-xl font-semibold">Native details</h2>
        <pre className="overflow-auto whitespace-pre-wrap break-all rounded border border-line bg-panel p-4 text-xs">
          {JSON.stringify(value, null, 2)}
        </pre>
      </section>
    </div>
  );
}
function Items({ data }: { data: Catalogue }) {
  const hash = useHash(),
    collections = useMemo(() => groups(data), [data]),
    controls = useMemo(
      () => new Map(data.controls.map((v) => [v.id, v])),
      [data],
    );
  const index = useMemo(
    () =>
      new Map(
        [
          ...Object.values(collections).flat(),
          ...Object.values(data.selections),
        ].map((v) => [v.id, v]),
      ),
    [collections, data],
  );
  const params = new URLSearchParams(hash.split("?")[1] || ""),
    requested = params.get("collection") || "",
    collection = Object.hasOwn(collections, requested)
      ? requested
      : data.items.length
        ? "items"
        : collections.clothing?.length
          ? "clothing"
          : "props";
  const [query, setQuery] = useState(params.get("q") || ""),
    [page, setPage] = useState(0),
    [lastBrowse, setLastBrowse] = useState(
      hash.startsWith("#entry/") ? "#" : hash || "#",
    );
  useEffect(() => {
    if (!hash.startsWith("#entry/")) {
      setQuery(new URLSearchParams(hash.split("?")[1] || "").get("q") || "");
      setPage(0);
      setLastBrowse(hash || "#");
      document.title = "Ghostline Items";
    }
  }, [hash]);
  const cat = params.get("category") || "",
    representation =
      collection === "props" ? params.get("representation") || "" : "",
    rendered = params.get("ready") === "1";
  const submittedQuery = params.get("q") || "";
  const rows = useMemo(
      () =>
        filterEntries(
          collections[collection] || [],
          data,
          controls,
          submittedQuery,
          cat,
          rendered,
          representation,
        ),
      [
        collections,
        collection,
        data,
        controls,
        submittedQuery,
        cat,
        rendered,
        representation,
      ],
    ),
    pages = Math.ceil(rows.length / 30),
    currentPage = Math.min(page, Math.max(0, pages - 1));
  const navigate = (updates: Record<string, string>): void => {
    const next = new URLSearchParams({
      collection,
      q: query,
      category: cat,
      representation,
      ready: rendered ? "1" : "",
      ...updates,
    });
    const nextHash = "#browse?" + next;
    if (nextHash === hash) setPage(0);
    else location.hash = nextHash;
  };
  let detailId: string | undefined;
  if (hash.startsWith("#entry/")) {
    try {
      detailId = decodeURIComponent(hash.slice(7));
    } catch {
      detailId = "";
    }
  }
  return (
    <AppShell
      product="Items"
      homeHref="#"
      actions={
        <a href="data/catalogue.json" className="text-accent">
          Catalogue JSON ↗
        </a>
      }
    >
      <main className="mx-auto w-full max-w-[1500px] space-y-6 p-4 sm:p-8">
        {detailId !== undefined ? (
          <>
            <a href={lastBrowse} className="text-accent">
              ← Catalogue
            </a>
            <Detail
              key={detailId}
              entry={index.get(detailId)}
              data={data}
              index={index}
              controls={controls}
            />
          </>
        ) : (
          <>
            <h1 className="text-4xl font-semibold">Items catalogue</h1>
            <p className="text-muted">
              Equipment, character customization, furniture and props.
            </p>
            <p className="text-sm text-muted" aria-live="polite">
              {data.items.length.toLocaleString()} records ·{" "}
              {(data.props?.length || 0).toLocaleString()} props ·{" "}
              {data.controls.length.toLocaleString()} controls ·{" "}
              {data.choices.length.toLocaleString()} choices ·{" "}
              {Object.keys(data.assets).length.toLocaleString()} resources
            </p>
            <form
              role="search"
              className="space-y-3"
              onSubmit={(e) => {
                e.preventDefault();
                navigate({ q: query });
              }}
            >
              <label htmlFor="search" className="text-sm">
                Search names, record IDs, appearances or mesh paths
              </label>
              <div className="flex gap-3">
                <Input
                  id="search"
                  type="search"
                  placeholder="A jacket, hairstyle, mesh or record ID…"
                  autoComplete="off"
                  value={query}
                  onChange={(e) => setQuery(e.target.value)}
                />
                <Button type="submit" variant="primary">
                  Search
                </Button>
              </div>
            </form>
            <div className="flex flex-wrap gap-4">
              <label className="grid gap-2 text-sm">
                Collection
                <Select
                  aria-label="Collection"
                  value={collection}
                  onChange={(e) =>
                    navigate({
                      collection: e.target.value,
                      category: "",
                      representation: "",
                    })
                  }
                >
                  {[
                    ["items", "Equipment"],
                    ["clothing", "Clothing (player & NPC)"],
                    ["props", "Furniture & props"],
                    ["controls", "Customization controls"],
                    ["choices", "Customization choices"],
                    ["assets", "Visual resources"],
                  ].map(([value, title]) => (
                    <option key={value} value={value}>
                      {title}
                    </option>
                  ))}
                </Select>
              </label>
              {collection === "props" && (
                <label className="grid gap-2 text-sm">
                  Representation
                  <Select
                    value={representation}
                    onChange={(e) =>
                      navigate({ representation: e.target.value })
                    }
                  >
                    <option value="">Assemblies and meshes</option>
                    <option value="assembly">Complete assemblies</option>
                    <option value="mesh">Individual meshes</option>
                  </Select>
                </label>
              )}
              <label className="grid gap-2 text-sm">
                Category / native slot
                <Select
                  value={cat}
                  onChange={(e) => navigate({ category: e.target.value })}
                >
                  <option value="">All</option>
                  {[
                    ...new Set(
                      (collections[collection] || []).map((v) =>
                        category(v, controls),
                      ),
                    ),
                  ]
                    .sort()
                    .map((c) => (
                      <option key={c}>{c}</option>
                    ))}
                </Select>
              </label>
              <label className="flex items-center gap-2 text-sm">
                <input
                  type="checkbox"
                  checked={rendered}
                  onChange={(e) =>
                    navigate({ ready: e.target.checked ? "1" : "" })
                  }
                />
                With rendered previews
              </label>
            </div>
            <div className="flex justify-between text-sm">
              <p role="status">{rows.length.toLocaleString()} entries</p>
              <a href="data/coverage.json" className="text-accent">
                Coverage report ↗
              </a>
            </div>
            <div className="grid grid-cols-1 gap-5 sm:grid-cols-2 xl:grid-cols-3">
              {rows.slice(currentPage * 30, (currentPage + 1) * 30).map((v) => {
                const media = previews(v, data),
                  hero = media.find((m) => m.view === "hero") || media[0];
                return (
                  <a
                    key={v.id}
                    href={entryHref(v.id)}
                    className="overflow-hidden rounded-xl border border-line bg-panel hover:border-accent"
                  >
                    <div className="flex aspect-square items-center justify-center bg-black/20">
                      {hero ? (
                        <img
                          className="h-full w-full object-contain"
                          src={
                            (globalThis.GHOSTLINE_MEDIA_BASE
                              ? globalThis.GHOSTLINE_MEDIA_BASE.replace(
                                  /\/$/,
                                  "",
                                ) + "/"
                              : "") + hero.path
                          }
                          alt={`${label(v)} — ${hero.view}`}
                          loading="lazy"
                        />
                      ) : (
                        <span className="p-5 text-sm text-muted">
                          {selectionIds(v, data).length
                            ? "Preview " +
                              (data.selections[selectionIds(v, data)[0]!]
                                ?.preview_state || "pending")
                            : "Native metadata"}
                        </span>
                      )}
                    </div>
                    <div className="space-y-3 p-5">
                      {["assembly", "mesh"].includes(v.kind || "") && (
                        <p className="text-xs text-accent">
                          {v.kind === "assembly"
                            ? "Complete assembly"
                            : "Individual mesh"}
                        </p>
                      )}
                      <p className="text-xs text-muted">
                        {category(v, controls)}
                      </p>
                      <h2 className="text-xl font-semibold">{label(v)}</h2>
                      <p className="break-all text-xs text-muted">
                        {technical(v)}
                      </p>
                    </div>
                  </a>
                );
              })}
            </div>
            {!rows.length && (
              <p className="py-12 text-center text-muted">
                No matching entries. Try another collection or a broader search.
              </p>
            )}
            <nav
              aria-label="Results pages"
              className="flex items-center justify-center gap-4"
            >
              <Button
                disabled={currentPage === 0}
                onClick={() => setPage(currentPage - 1)}
              >
                ← Previous
              </Button>
              <span>
                {rows.length ? currentPage + 1 : 0} / {pages}
              </span>
              <Button
                disabled={currentPage + 1 >= pages}
                onClick={() => setPage(currentPage + 1)}
              >
                Next →
              </Button>
            </nav>
          </>
        )}
      </main>
      <footer className="mt-auto flex flex-wrap justify-between gap-3 border-t border-line p-6 text-xs text-muted">
        <span>Ghostline / Open asset reference</span>
        <span>Cyberpunk 2077 {data.game_version || ""}</span>
      </footer>
    </AppShell>
  );
}
const data = globalThis.GHOSTLINE_CATALOGUE;
createRoot(document.getElementById("root")!).render(
  data ? (
    <Items data={data} />
  ) : (
    <p role="alert">Catalogue data could not be loaded.</p>
  ),
);
