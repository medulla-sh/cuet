package google

import (
	T "github.com/medulla-sh/cuet"
)

#ArtifactRegistryMultiRegion: "us" | "europe" | "asia"
#ArtifactRegistryLocation:    #Region | #ArtifactRegistryMultiRegion

#ArtifactRegistryFormat:
	"DOCKER" |
	"MAVEN" |
	"NPM" |
	"APT" |
	"YUM" |
	"GO" |
	"KFP"

#ArtifactRegistry: {
	in: {
		#import?: string

		name: string

		repositoryId: string
		repositoryId: _ | *name

		location: #ArtifactRegistryLocation

		format: #ArtifactRegistryFormat
		format: _ | *"DOCKER"

		description?: string

		iam: [string]: {
			#import?: string

			role:   string
			member: string
		}

		*{} | {
			format: "DOCKER"
			dockerConfig: {
				immutableTags: bool
			}
		} | {
			format: "DOCKER"
			remoteConfig: {
				description?:     string
				publicRepository: "DOCKER_HUB"
			}
		}
	}

	ref: "google_artifact_registry_repository.\(in.name)"
	out: T.#TerraformInput & {
		resource: google_artifact_registry_repository: (in.name): {
			if in.#import != _|_ {
				#import: in.#import
			}

			repository_id: in.repositoryId
			location:      in.location
			format:        in.format

			if in.description != _|_ {
				description: in.description
			}

			if in.dockerConfig != _|_ {
				docker_config: {
					immutable_tags: in.dockerConfig.immutableTags
				}
			}

			if in.remoteConfig != _|_ {
				mode: "REMOTE_REPOSITORY"
				remote_repository_config: {
					if in.remoteConfig.description != _|_ {
						description: in.remoteConfig.description
					}
					docker_repository: public_repository: in.remoteConfig.publicRepository
				}
			}
		}

		for bindingName, binding in in.iam {
			let resourceName = "\(in.name)-\(bindingName)"
			resource: google_artifact_registry_repository_iam_member: (resourceName): {
				if binding.#import != _|_ {
					#import: binding.#import
				}

				project:    "${\(ref).project}"
				location:   "${\(ref).location}"
				repository: "${\(ref).repository_id}"
				role:       binding.role
				member:     binding.member
			}
		}
	}
}
