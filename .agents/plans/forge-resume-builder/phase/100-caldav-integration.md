# Phase 100: CalDAV Integration

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans

**Depends on:** None
**Blocks:** Nothing
**Parallelizable with:** All phases
**Duration:** Medium-Long (5 tasks)

## Goal

Add CalDAV client support for read-write calendar integration. Connect to external calendars, pull events, create events from Forge (interview scheduling, application deadlines). Phase 1: basic CalDAV client with event CRUD. Sync, conflict resolution, and recurring events deferred.

## Non-Goals

- CalDAV server (Forge is a client only)
- Recurring event support (Phase 1)
- Conflict resolution for concurrent edits
- Calendar sharing / multi-user sync
- Google Calendar API (use CalDAV protocol for broad compatibility)

---

## Tasks

### T100.1: CalDAV Client Library

**Steps:**
1. Add CalDAV client dependency (e.g., `tsdav` or implement minimal CalDAV/WebDAV over HTTP)
2. Create `packages/core/src/services/caldav-service.ts`:
   - `connect(serverUrl, username, password)` — discover calendars
   - `listCalendars()` — return available calendars
   - `listEvents(calendarId, dateRange)` — fetch events in range
   - `createEvent(calendarId, event)` — create iCal event
   - `updateEvent(calendarId, eventId, event)` — update event
   - `deleteEvent(calendarId, eventId)` — delete event
3. Store CalDAV connection config (server URL, credentials) — encrypted in user_profile or separate config table
4. iCal event format: VEVENT with summary, dtstart, dtend, description, location

**Acceptance Criteria:**
- [ ] Can connect to a CalDAV server and list calendars
- [ ] Can CRUD events
- [ ] Credentials stored securely

### T100.2: Forge Event Types

**Steps:**
1. Define Forge event types that map to calendar events:
   - Interview (JD link, datetime, location/video link, interviewer contacts)
   - Application Deadline (JD link, deadline date)
   - Follow-up Reminder (JD link, date, notes)
   - Custom Event (free-form)
2. Create `forge_events` table: id, event_type, title, description, start_at, end_at, jd_id FK, calendar_event_uid, synced_at
3. Two-way mapping: Forge event ↔ CalDAV VEVENT via UID

**Acceptance Criteria:**
- [ ] Event types defined
- [ ] `forge_events` table created
- [ ] Bidirectional UID mapping

### T100.3: API & SDK

**Steps:**
1. Routes: `/api/calendar/connect`, `/api/calendar/calendars`, `/api/calendar/events` (CRUD)
2. Routes: `/api/events` for Forge-side event management
3. SDK: CalendarResource with connect, listCalendars, listEvents, createEvent, etc.
4. Sync endpoint: `/api/calendar/sync` — pull remote events, push local events

**Acceptance Criteria:**
- [ ] Calendar connection flow works
- [ ] Event CRUD through API
- [ ] Basic sync (pull then push)

### T100.4: WebUI — Calendar View

**Steps:**
1. Add calendar view page (or integrate into Dashboard)
2. Month/week view showing events from connected calendars + Forge events
3. Click on JD → create interview/deadline event
4. Event creation modal with type selector, date/time picker, JD linker
5. Visual distinction between Forge-created events and external events
6. Navigation in config: Integrations page links to calendar setup

**Acceptance Criteria:**
- [ ] Calendar renders events in month/week view
- [ ] Can create events from JD context
- [ ] Event modal works
- [ ] External vs Forge events visually distinct

### T100.5: Tests

**Acceptance Criteria:**
- [ ] CalDAV client connects to mock server
- [ ] Event CRUD roundtrip works
- [ ] Forge event ↔ iCal mapping correct
- [ ] UI renders events
