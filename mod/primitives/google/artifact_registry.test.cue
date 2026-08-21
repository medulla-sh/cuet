@if(test)

package google

#ArtifactRegistryTests: {
	"Docker remote repository": {
		repository: #ArtifactRegistry & {in: {
			name:        "docker-hub"
			location:    "us-west1"
			description: "Docker Hub cache"
			remoteConfig: {
				description:      "Docker Hub upstream"
				publicRepository: "DOCKER_HUB"
			}
		}}

		assert: repository.out.resource.google_artifact_registry_repository["docker-hub"] == {
			repository_id: "docker-hub"
			location:      "us-west1"
			format:        "DOCKER"
			description:   "Docker Hub cache"
			mode:          "REMOTE_REPOSITORY"
			remote_repository_config: {
				description: "Docker Hub upstream"
				docker_repository: public_repository: "DOCKER_HUB"
			}
		}
	}
	"reject mixed repository modes": {
		repository: {
			in: {
				name:     "mixed"
				location: "us-west1"
				dockerConfig: immutableTags:    true
				remoteConfig: publicRepository: "DOCKER_HUB"
			}
		}

		assert: (#ArtifactRegistry & repository) == _|_
	}
}

artifactRegistryResult: [for _, test in #ArtifactRegistryTests {test.assert & true}]
