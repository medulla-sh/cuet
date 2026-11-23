package google

import (
	"encoding/hex"

	T "github.com/medulla-sh/cuet"
)

#Folder: {
	in: {
		#import?: string
		#dependsOn?: [...string]

		name:        string
		displayName: string
		displayName: _ | *name
		// Accept resource names and Terraform expressions for nested folders.
		parent: string & !=""

		deletionProtection: bool
		deletionProtection: _ | *true
	}

	ref: "google_folder.\(in.name)"
	out: T.#TerraformInput & {
		resource: google_folder: (in.name): {
			if in.#import != _|_ {
				#import: in.#import
			}
			if in.#dependsOn != _|_ {
				#dependsOn: in.#dependsOn
			}
			display_name:        in.displayName
			parent:              in.parent
			deletion_protection: in.deletionProtection
		}
	}
}

// #Folders creates a hierarchy below an organization or folder. Use _ or {}
// for a leaf. Refs are keyed by slash-separated paths relative to the parent.
#Folders: {
	in: {
		parent: string & !=""
		tree:   _#FolderTree
	}

	let rendered = _#RenderFolders & {"in": {
		parent:      in.parent
		tree:        in.tree
		pathPrefix:  ""
		labelPrefix: "folder-"
	}}
	refs: rendered.refs
	out:  T.#TerraformInput & rendered.out
}

_#FolderTree: {
	[string & !="" & !~"/"]: _#FolderTree
}

_#RenderFolders: {
	in: {
		parent:      string
		tree:        _#FolderTree
		pathPrefix:  string
		labelPrefix: string
	}

	let nodes = {
		for display, subtree in in.tree {
			// Reserve "--" for path boundaries and "_" for escaped names.
			let plain = display =~ "^[A-Za-z0-9]+(-[A-Za-z0-9]+)*$"
			let segment = [
				if plain {display},
				if !plain {"x_\(hex.Encode(display))"},
			][0]
			let path = in.pathPrefix + display
			let label = in.labelPrefix + segment
			let folder = #Folder & {"in": {
				name:        label
				displayName: display
				parent:      in.parent
			}}
			(display): {
				refs: (path): folder.ref
				out: folder.out
				let children = _#RenderFolders & {"in": {
					parent:      "${\(folder.ref).name}"
					tree:        subtree
					pathPrefix:  "\(path)/"
					labelPrefix: "\(label)--"
				}}
				refs: children.refs
				out:  children.out
			}
		}
	}
	refs: {
		for _, node in nodes {node.refs}
	}
	out: {
		for _, node in nodes {node.out}
	}
}
