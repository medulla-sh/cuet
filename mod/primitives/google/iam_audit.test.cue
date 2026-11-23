@if(test)

package google

#OrgIamAuditConfigTest: {
	input: #OrgIamAuditConfig & {in: {
		name:    "secret-manager"
		orgId:   "123"
		service: "secretmanager.googleapis.com"
		logs: {
			ADMIN_READ: []
			DATA_READ: ["user:example@example.com"]
		}
	}}

	config: input.out.resource.google_organization_iam_audit_config["secret-manager"]
	assert: config.audit_log_config == [
		{log_type: "ADMIN_READ"},
		{log_type: "DATA_READ", exempted_members: ["user:example@example.com"]},
	]
}

orgIamAuditConfigResult: #OrgIamAuditConfigTest.assert & true
