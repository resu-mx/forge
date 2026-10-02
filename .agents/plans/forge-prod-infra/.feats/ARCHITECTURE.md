---
status: draft
wip: true
---
# Forge Prod Architecture

## Deployment Model

### Overview

```asciidoc
client -> CDN -> CF -> DB
                    -> 
                    -> 
```

### 

```asciidoc
webui -> api -> CDN -> CF -> DB
  tui -> 
  cli -> 
  mcp -> 
```

## Data Model

`private -> controlled -> public`

```asciidoc
- private
  - ssn
  - private-creds
- controlled
  - 
- public
```

## Application Layers

- routing layer ()
- service layer ()
- integrity layer (`EntityLifecycleManager`)
- adapter layer ()
- storage layer ()

```asciidoc
ui -> route-a -> service-a (API?) -> EntityLifecycleManager -> adapter-X -> DB
```
