# Profile

## Schema: `Profile`

```json
{
  "name": {
    "first": "<String>",
    "middle": "Option<Vec<String>>",
    "last": "<String>",
  },
  "email": "<String::email>",
  "phone": "<String::phone>",
  "location": "<Location>",
  "website": { // HashMap <String,String::url>
    "linkedin": "<String::url::linkedin>",
    "github": "<String::url::github>",
    "blog": "<String::url>",
    "portfolio": "<String::url>",
    "<foo>": "<String::url>"
  },
  "salary expectations": {
    "minimum": "<Int>",
    "target": "<Int>",
    "stretch": "<Int>"
  }
}
```
