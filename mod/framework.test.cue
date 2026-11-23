@if(test)

package cuet

let resourceMove = _#GenerateMoves & {in: {
	#module:  "infra/gcp/access"
	#envName: "global"
	resource: google_organization_iam_member: admin: {
		#history: [{module: "infra/gcp"}]
	}
}}

crossStateMoveResult: (resourceMove.crossStateTransitions == [{
	resourceType: "google_organization_iam_member"
	from: {module: "infra/gcp", env: "global", name: "admin"}
	to: {module: "infra/gcp/access", env: "global", name: "admin"}
}]) & true
