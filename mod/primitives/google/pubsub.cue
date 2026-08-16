package google

import T "github.com/medulla-sh/cuet"

#PubSubDuration: =~"^[0-9]+(\\.[0-9]+)?s$"

#PubSubTopic: {
	in: {
		name: string
		project: {
			name: string
			id?:  string
		}
	}

	ref: "google_pubsub_topic.\(in.name)"
	out: T.#TerraformInput & {
		data: google_project: (in.project.name): {
			if in.project.id != _|_ {
				project_id: in.project.id
			}
		}
		resource: google_pubsub_topic: (in.name): {
			name:    in.name
			project: "${data.google_project.\(in.project.name).project_id}"
		}
	}
}

#PubSubPushIdentity: {
	in: {
		accountId: string

		name: string
		name: _ | *accountId

		displayName: string
		displayName: _ | *accountId

		description?: string
		project: {
			name: string
			id?:  string
		}
	}

	let pubsubServiceIdentity = #ProjectServiceIdentity & {"in": {
		name:    "\(in.project.name)_pubsub"
		service: "pubsub.googleapis.com"
		project: in.project
	}}
	let pushServiceAccount = #ServiceAccount & {"in": {
		accountId:   in.accountId
		name:        in.name
		displayName: in.displayName
		if in.description != _|_ {
			description: in.description
		}
		project: in.project
		iam: "token-creator": {
			role:   "roles/iam.serviceAccountTokenCreator"
			member: "${\(pubsubServiceIdentity.ref).member}"
		}
	}}

	ref: pushServiceAccount.ref
	out: pubsubServiceIdentity.out & pushServiceAccount.out
}

#PubSubPushSubscription: {
	in: {
		name:  string
		topic: string
		project: {
			name: string
			id?:  string
		}

		endpoint:            string & =~"^https://"
		serviceAccountEmail: string & !=""
		audience:            string & =~"^https://"
		audience:            _ | *endpoint

		ackDeadlineSeconds:       int & >=10 & <=600
		ackDeadlineSeconds:       _ | *10
		messageRetentionDuration: #PubSubDuration
		messageRetentionDuration: _ | *"604800s"
		expirationTtl:            "" | #PubSubDuration
		expirationTtl:            _ | *""
		minimumBackoff:           #PubSubDuration
		minimumBackoff:           _ | *"10s"
		maximumBackoff:           #PubSubDuration
		maximumBackoff:           _ | *"600s"
	}

	ref: "google_pubsub_subscription.\(in.name)"
	out: T.#TerraformInput & {
		data: google_project: (in.project.name): {
			if in.project.id != _|_ {
				project_id: in.project.id
			}
		}
		resource: google_pubsub_subscription: (in.name): {
			name:                       in.name
			project:                    "${data.google_project.\(in.project.name).project_id}"
			topic:                      in.topic
			ack_deadline_seconds:       in.ackDeadlineSeconds
			message_retention_duration: in.messageRetentionDuration
			expiration_policy: ttl: in.expirationTtl
			retry_policy: {
				minimum_backoff: in.minimumBackoff
				maximum_backoff: in.maximumBackoff
			}
			push_config: {
				push_endpoint: in.endpoint
				no_wrapper: write_metadata: false
				oidc_token: {
					service_account_email: in.serviceAccountEmail
					audience:              in.audience
				}
			}
		}
	}
}
