@if(test)

package google

#PubSubTests: {
	"topic": {
		input: #PubSubTopic & {in: {
			name: "gcr"
			project: {
				name: "internal"
				id:   "oakmont-internal"
			}
		}}

		assert: input.out.resource.google_pubsub_topic.gcr == {
			name:    "gcr"
			project: "${data.google_project.internal.project_id}"
		}
	}

	"push-identity": {
		input: #PubSubPushIdentity & {in: {
			name:        "artifact_registry_events"
			accountId:   "artifact-registry-events"
			displayName: "Artifact Registry event delivery"
			project: {
				name: "internal"
				id:   "oakmont-internal"
			}
		}}

		let iam = input.out.resource.google_service_account_iam_member["artifact_registry_events-token-creator"]
		assert: input.ref == "google_service_account.artifact_registry_events"
		assert: iam == {
			service_account_id: "${google_service_account.artifact_registry_events.name}"
			role:               "roles/iam.serviceAccountTokenCreator"
			member:             "${google_project_service_identity.internal_pubsub.member}"
		}
	}

	"push-subscription": {
		input: #PubSubPushSubscription & {in: {
			name:  "flux-dev"
			topic: "projects/example/topics/gcr"
			project: name: "internal"
			endpoint:            "https://flux.example/hook/token"
			serviceAccountEmail: "events@example.iam.gserviceaccount.com"
		}}

		let subscription = input.out.resource.google_pubsub_subscription["flux-dev"]
		assert: subscription.push_config == {
			push_endpoint: "https://flux.example/hook/token"
			no_wrapper: write_metadata: false
			oidc_token: {
				service_account_email: "events@example.iam.gserviceaccount.com"
				audience:              "https://flux.example/hook/token"
			}
		}
		assert: subscription.retry_policy == {
			minimum_backoff: "10s"
			maximum_backoff: "600s"
		}
	}
}

pubSubResult: [for _, test in #PubSubTests {test.assert & true}]
