# Importing

`cuet` supports Terraform imports through the `#import` meta field on resource
primitives.

When present, `cuet` emits Terraform `import` entries in generated JSON.

## Basic example

```cue
package thoth

import (
    "example.com/cuet/primitives/google"
    I "example.com/infra"
)

I.#InfraModule

infra: dev: {
    (google.#Bucket & {in: {
        #import:  "example-dev/example-assets-dev"
        name:     "example-assets-dev"
        location: "us-west1"
    }}).out
}
```

This generates an import block equivalent to:

```hcl
import {
  to = google_storage_bucket.example-assets-dev
  id = "example-dev/example-assets-dev"
}
```

## Notes

- Imports are generated from resources, not data sources.
- Keep import IDs environment-correct (`dev`, `staging`, `prod`).
- Run `cuet ... tf plan` after import wiring to confirm state alignment.
