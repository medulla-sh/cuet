@if(test)

package google

#LoggingTests: {
	bucket: #LoggingBucket & {in: {
		name:          "archive"
		projectId:     "${google_project.archive.project_id}"
		location:      "us-west1"
		retentionDays: 2192
		#dependsOn: ["google_project_service.logging"]
	}}
	sink: #OrgLoggingSink & {in: {
		name:            "org-audit"
		orgId:           "123"
		destination:     bucket.destination
		filter:          #"LOG_ID("cloudaudit.googleapis.com/activity")"#
		includeChildren: true
	}}

	assert: [
		bucket.out.resource.google_logging_project_bucket_config.archive.#dependsOn == ["google_project_service.logging"],
		sink.out.resource.google_logging_organization_sink["org-audit"].destination == "logging.googleapis.com/${google_logging_project_bucket_config.archive.name}",
		sink.out.resource.google_logging_organization_sink["org-audit"].intercept_children == false,
		sink.writerIdentity == "${google_logging_organization_sink.org-audit.writer_identity}",
	]
}

loggingResult: [for assertion in #LoggingTests.assert {assertion & true}]
