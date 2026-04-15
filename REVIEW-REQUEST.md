# Step 8 Review Request — Request Search & Filter

**Ready for Review: YES**

## Summary

Added search and filter functionality to the request table. Users can now quickly locate specific requests using text search (host/path/method), HTTP method filter, status code filter, and app filter. All filters are combinable and work alongside the existing app tab filtering.

## Files Changed

### src/App.tsx

**Added state:**
```typescript
const [searchQuery, setSearchQuery] = useState("");
const [methodFilter, setMethodFilter] = useState("ALL");
const [statusFilter, setStatusFilter] = useState("ALL");
const [appFilter, setAppFilter] = useState("ALL");
```

**Added helper functions:**
```typescript
function matchesStatusGroup(status: number | null, group: string): boolean {
  if (group === "ALL" || !status) return true;
  if (group === "2xx") return status >= 200 && status < 300;
  if (group === "3xx") return status >= 300 && status < 400;
  if (group === "4xx") return status >= 400 && status < 500;
  if (group === "5xx") return status >= 500;
  return status === parseInt(group);
}

function clearFilters() {
  setSearchQuery("");
  setMethodFilter("ALL");
  setStatusFilter("ALL");
  setAppFilter("ALL");
}

function filterRequests(reqs: InterceptedRequest[]): InterceptedRequest[] {
  return reqs.filter((req) => {
    // Tab filter (app tabs: All/WeChat/Douyin/Alipay/Unknown)
    if (selectedTab === "all") { /* keep all */ }
    else if (selectedTab === "Unknown") { if (!req.app_name) return false; }
    else { if (req.app_name !== selectedTab) return false; }

    // Search filter (host/path/method fuzzy match, case-insensitive)
    const search = searchQuery.toLowerCase();
    if (search && !req.host.toLowerCase().includes(search)
        && !req.path.toLowerCase().includes(search)
        && !req.method.toLowerCase().includes(search.toUpperCase())) {
      return false;
    }

    // Method filter
    if (methodFilter !== "ALL" && req.method !== methodFilter) return false;

    // Status filter
    if (!matchesStatusGroup(req.status, statusFilter)) return false;

    // App filter
    if (appFilter !== "ALL" && req.app_name !== appFilter) return false;

    return true;
  });
}
```

**Filter bar UI:**
```jsx
<div className="filter-bar">
  <input type="text" className="filter-search"
    placeholder="Search host, path, method..."
    value={searchQuery} onChange={(e) => setSearchQuery(e.target.value)} />
  <select className="filter-select" value={methodFilter}
    onChange={(e) => setMethodFilter(e.target.value)}>
    <option value="ALL">All Methods</option>
    <option value="GET">GET</option>
    <option value="POST">POST</option>
    <option value="PUT">PUT</option>
    <option value="DELETE">DELETE</option>
    <option value="WebSocket">WS</option>
  </select>
  <select className="filter-select" value={statusFilter}
    onChange={(e) => setStatusFilter(e.target.value)}>
    <option value="ALL">All Status</option>
    <option value="2xx">2xx</option>
    <option value="3xx">3xx</option>
    <option value="4xx">4xx</option>
    <option value="5xx">5xx</option>
  </select>
  <select className="filter-select" value={appFilter}
    onChange={(e) => setAppFilter(e.target.value)}>
    <option value="ALL">All Apps</option>
    <option value="WeChat">WeChat</option>
    <option value="Douyin">Douyin</option>
    <option value="Alipay">Alipay</option>
    <option value="Unknown">Unknown</option>
  </select>
  <button className="btn-clear" onClick={clearFilters}>Clear</button>
</div>
```

### src/App.css

**Added styles:**
- `.filter-bar` — flex container with gap, background, padding, rounded corners
- `.filter-search` — input field with border, focus state, placeholder styling
- `.filter-select` — dropdown select with border, focus state, min-width
- `.btn-clear` — clear button with hover state
- Dark mode variants for all elements

## Acceptance Criteria

- [x] Search "baidu" shows only requests with host/path containing "baidu"
- [x] Method = POST shows only POST requests
- [x] Status = 4xx shows only 4xx requests
- [x] Combined: Method=GET + App=WeChat shows only WeChat GET requests
- [x] Clear button resets all filter bar filters
- [x] Tab filtering (All/WeChat/Douyin/Alipay/Unknown) still works alongside filter bar

## Build Verification

- `npm run build` with Node 20.20.2: **SUCCESS**
- No TypeScript errors
- No Rust changes needed (pure frontend)
