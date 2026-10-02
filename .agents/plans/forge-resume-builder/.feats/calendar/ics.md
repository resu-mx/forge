# ICS Export

## Goals

- Integrate w/ `Contact`
- Integrate w/ Emails
- Export/Attach `JD`, `Resume`, & `CoverLetter`
- Export/Attach `InterviewCliffNotes` (Created to assist during interview)
- Create/Associate a Special Note for tracking interview details/notes (taken during interview)

## Schema

```json
{
  "BEGIN":"VCALENDAR",
  "VERSION":"2.0",
  "CALSCALE":"GREGORIAN",
  "BEGIN":"VEVENT",
  "SUMMARY":"Access-A-Ride Pickup",
  "DTSTART": "TZID=America/New_York:20130802T103400",
  "DTEND": "TZID=America/New_York:20130802T110400",
  "LOCATION":"1000 Broadway Ave.\, Brooklyn",
  "DESCRIPTION":" Access-A-Ride to 900 Jay St.\, Brooklyn",
  "STATUS":"CONFIRMED",
  "SEQUENCE":"3",
  "BEGIN":"VALARM",
  "TRIGGER":"-PT10M",
  "DESCRIPTION":"Pickup Reminder",
  "ACTION":"DISPLAY",
  "END":"VALARM",
  "END":"VEVENT",
  "BEGIN":"VEVENT",
  "SUMMARY":"Access-A-Ride Pickup",
  "DTSTART": "TZID=America/New_York:20130802T200000",
  "DTEND": "TZID=America/New_York:20130802T203000",
  "LOCATION":"900 Jay St.\, Brooklyn",
  "DESCRIPTION":" Access-A-Ride to 1000 Broadway Ave.\, Brooklyn",
  "STATUS":"CONFIRMED",
  "SEQUENCE":"3",
  "BEGIN":"VALARM",
  "TRIGGER":"-PT10M",
  "DESCRIPTION":"Pickup Reminder",
  "ACTION":"DISPLAY",
  "END":"VALARM",
  "END":"VEVENT",
  "END":"VCALENDAR",
}
```

## Sources

- https://icscalendar.com/ics-calendar-settings/
- https://www.webdavsystem.com/server/creating_caldav_carddav/calendar_ics_file_structure/
- https://gist.github.com/DeMarko/6142417
- https://en.wikipedia.org/wiki/ICalendar
