@if(test)

package google

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
