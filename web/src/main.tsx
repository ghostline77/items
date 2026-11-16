import { AppShell, Button, Input, Select, mountApp } from "@ghostline/ui-core";
import "./style.css";

function ItemsCatalogue() {
  return (
    <AppShell
      product="Items"
      homeHref="#"
      actions={<a href="data/catalogue.json">Catalogue JSON ↗</a>}
    >
      <main>
        <section id="browse">
          <h1>Items catalogue</h1>
          <p className="intro">
            Equipment, character customization, furniture and props.
          </p>
          <p id="coverage" className="counts" aria-live="polite" />
          <form id="search-form" role="search">
            <label htmlFor="search">
              Search names, record IDs, appearances or mesh paths
            </label>
            <div className="search-row">
              <Input
                id="search"
                type="search"
                placeholder="A jacket, hairstyle, mesh or record ID…"
                autoComplete="off"
              />
              <Button type="submit" variant="primary">
                Search
              </Button>
            </div>
          </form>
          <div className="filters">
            <label>
              Collection
              <Select id="collection">
                <option value="items">Equipment</option>
                <option value="clothing">Clothing (player & NPC)</option>
                <option value="props">Furniture & props</option>
                <option value="controls">Customization controls</option>
                <option value="choices">Customization choices</option>
                <option value="assets">Visual resources</option>
              </Select>
            </label>
            <label id="prop-kind-label" hidden>
              Representation
              <Select id="prop-kind">
                <option value="">Assemblies and meshes</option>
                <option value="assembly">Complete assemblies</option>
                <option value="mesh">Individual meshes</option>
              </Select>
            </label>
            <label>
              Category / native slot
              <Select id="category">
                <option value="">All</option>
              </Select>
            </label>
            <label className="check">
              <Input type="checkbox" id="ready" /> With rendered previews
            </label>
          </div>
          <div className="result-bar">
            <p id="result-count" role="status" />
            <a href="data/coverage.json">Coverage report ↗</a>
          </div>
          <div id="results" className="grid" />
          <nav className="pager" aria-label="Results pages">
            <Button id="previous">← Previous</Button>
            <span id="page" />
            <Button id="next">Next →</Button>
          </nav>
        </section>
        <section id="detail" hidden>
          <a id="back" href="#">
            ← Catalogue
          </a>
          <div id="detail-content" />
        </section>
      </main>
      <footer>
        <span>Ghostline / Open asset reference</span>
        <span id="source-version" />
      </footer>
    </AppShell>
  );
}
mountApp(<ItemsCatalogue />, () => import("../app.js"));
