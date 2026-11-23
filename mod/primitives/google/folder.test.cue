@if(test)

package google

import (
	"list"
	"strings"
)

#FolderTests: {
	"defaults": {
		folder: #Folder & {in: {
			name:   "runtime"
			parent: "organizations/123"
		}}

		assert: folder.ref == "google_folder.runtime"
		assert: folder.out.resource.google_folder.runtime == {
			display_name:        "runtime"
			parent:              "organizations/123"
			deletion_protection: true
		}
	}
	"nested folder and project": {
		root: #Folder & {in: {
			name:   "runtime"
			parent: "organizations/123"
		}}
		child: #Folder & {in: {
			name:               "runtime-prod"
			displayName:        "prod"
			parent:             "${\(root.ref).name}"
			deletionProtection: false
		}}
		project: #Project & {in: {
			name:     "production"
			folderId: "${\(child.ref).folder_id}"
			enabledServices: ["cloudresourcemanager.googleapis.com"]
		}}
		output: {
			root.out
			child.out
			project.out
		}

		assert: output.resource.google_folder["runtime-prod"] == {
			display_name:        "prod"
			parent:              "${google_folder.runtime.name}"
			deletion_protection: false
		}
		assert: output.resource.google_project.production.folder_id == "${google_folder.runtime-prod.folder_id}"
	}
	"framework controls": {
		folder: #Folder & {in: {
			name:    "prod"
			parent:  "folders/123"
			#import: "folders/456"
			#dependsOn: ["google_folder.runtime"]
		}}

		assert: folder.out.resource.google_folder.prod.#import == folder.in.#import
		assert: folder.out.resource.google_folder.prod.#dependsOn == folder.in.#dependsOn
	}
}

folderResult: [for _, test in #FolderTests {test.assert & true}]

let hierarchy = #Folders & {in: {
	parent: "folders/123"
	tree: {
		runtime: dev: _
		platform: dev: {}
		"runtime--dev": _
		"Shared Services": tooling: dev: _
	}
}}
let hierarchyResources = hierarchy.out.resource.google_folder

folderHierarchyResult: [
	len(hierarchy.refs) & 8,
	len(hierarchyResources) & 8,
	(hierarchy.refs["runtime/dev"] != hierarchy.refs["platform/dev"]) & true,
	(hierarchy.refs["runtime/dev"] != hierarchy.refs["runtime--dev"]) & true,
	for path, ref in hierarchy.refs {
		let labels = strings.Split(path, "/")
		let resource = hierarchyResources[strings.TrimPrefix(ref, "google_folder.")]
		let parent = [
			if len(labels) == 1 {"folders/123"},
			if len(labels) > 1 {"${\(hierarchy.refs[strings.Join(list.Take(labels, len(labels)-1), "/")]).name}"},
		][0]
		(resource.parent == parent && resource.display_name == labels[len(labels)-1] && resource.deletion_protection) & true
	},
]
