@if(test)

package google

googleDependencyResults: {
	let testProject = #Project & {in: {
		name: "audit"
		enabledServices: ["iam.googleapis.com", "alloydb.googleapis.com", "storage.googleapis.com"]
	}}
	let dependencies = [testProject.refs.services["iam.googleapis.com"]]
	let account = #ServiceAccount & {in: {
		#dependsOn: dependencies
		accountId:  "audit"
		project: name: "audit"
		iam: audit: {role: "roles/iam.workloadIdentityUser", member: "user:example@example.com"}
	}}
	let pool = #WorkloadIdentityPool & {in: {
		#dependsOn: dependencies
		name:       "audit-pool"
		project: name: "audit"
	}}
	let provider = #WorkloadIdentityProvider & {in: {
		#dependsOn: dependencies
		name:       "audit-provider"
		project: name: "audit"
		poolId: "${\(pool.ref).workload_identity_pool_id}"
		aws: accountId:                     "123456789012"
		attributeMapping: "google.subject": "assertion.arn"
		attributeCondition: "assertion.account == '123456789012'"
	}}
	let testMember = #ServiceAccountIamMember & {in: {
		#dependsOn:       dependencies
		name:             "audit"
		serviceAccountId: "${\(account.ref).name}"
		role:             "roles/iam.workloadIdentityUser"
		member:           "user:example@example.com"
	}}

	accountDependency:  (account.out.resource.google_service_account.audit.#dependsOn == dependencies) & true
	poolDependency:     (pool.out.resource.google_iam_workload_identity_pool["audit-pool"].#dependsOn == dependencies) & true
	providerDependency: (provider.out.resource.google_iam_workload_identity_pool_provider["audit-provider"].#dependsOn == dependencies) & true
	memberDependency:   (testMember.out.resource.google_service_account_iam_member.audit.#dependsOn == dependencies) & true
	lookupUnaffected:   (account.out.data.google_project.audit.#dependsOn == _|_) & true
	membershipImplicit: (account.out.resource.google_service_account_iam_member["audit-audit"].service_account_id == "${google_service_account.audit.name}") & true
}
