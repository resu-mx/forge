# Phase 99: Choropleth Enhancement — City/Zipcode Granularity

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans

**Depends on:** None (enhances Phase 68 choropleth)
**Blocks:** Nothing
**Parallelizable with:** All phases
**Duration:** Medium (4 tasks)

## Goal

Enhance the US choropleth map from state-level granularity to city/zipcode level with hexagonal binning. Show role saturation (JD density) at finer geographic resolution.

## Non-Goals

- International maps
- Real-time JD location updates
- Integration with external job board APIs for location data

---

## Tasks

### T99.1: Location Data Enrichment

**Steps:**
1. Audit JD location data — what format are locations stored in? (free text, city+state, zipcode?)
2. If free text: implement a location normalizer (parse "Remote", "Washington, DC", "San Francisco, CA 94105" into structured city/state/zip)
3. Geocode normalized locations to lat/lng (use a static lookup table for US cities/zips, not an API)
4. Store enriched location data on JDs (or in a cache table)

**Acceptance Criteria:**
- [ ] JD locations parsed into structured city/state/zip
- [ ] Lat/lng available for map rendering
- [ ] "Remote" and unknown locations handled gracefully

### T99.2: Hexagonal Binning Implementation

**Steps:**
1. Implement hex binning algorithm: divide map area into hexagonal cells
2. Aggregate JD counts per hex cell
3. Color hex cells by density (gradient: low=cool, high=warm)
4. Cell size configurable (zoom-dependent or fixed)
5. Use ECharts custom series or a geo library (d3-hexbin) for hex rendering

**Acceptance Criteria:**
- [ ] Hex bins render on map with density coloring
- [ ] Bin size produces readable visualization at default zoom
- [ ] Gradient color scale present

### T99.3: Map Interaction & Zoom

**Steps:**
1. Zoom: deeper zoom shows finer hex resolution
2. Hover: tooltip showing hex cell JD count, representative city names
3. Click: filter JD list to jobs in that hex cell
4. Toggle between state-level (existing) and hex-level views
5. Handle "Remote" jobs separately (sidebar count or special indicator)

**Acceptance Criteria:**
- [ ] Zoom changes granularity
- [ ] Hover and click interactions work
- [ ] Toggle between state and hex views
- [ ] Remote jobs handled

### T99.4: Tests

**Acceptance Criteria:**
- [ ] Location normalizer parses common formats
- [ ] Hex binning produces correct cell assignments
- [ ] Map renders without errors with sample data
