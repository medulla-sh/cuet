@if(test)

package google

#CloudSqlTests: {
	"postgres-instance": {
		input: #CloudSqlInstance & {in: {
			#import: {
				instance: "projects/example/instances/dagster-dev"
				databases: analytics: "example/dagster-dev/analytics"
			}
			name:   "dagster-dev"
			region: "us-west1"
			engine: {
				type:    "postgres"
				version: 17
			}
			tier: "db-f1-micro"
			privateNetwork: {
				id:             "projects/example/global/networks/dev"
				allocatedRange: "google-services"
			}
			databases: {
				dagster: {}
				analytics: {}
			}
			users: {
				dagster: password:  "${random_password.dagster.result}"
				readonly: password: "${random_password.readonly.result}"
			}
		}}

		assert: input.out.resource.google_sql_database_instance["dagster-dev"] == {
			#import:             "projects/example/instances/dagster-dev"
			name:                "dagster-dev"
			region:              "us-west1"
			database_version:    "POSTGRES_17"
			deletion_protection: true
			settings: {
				tier:                        "db-f1-micro"
				edition:                     "ENTERPRISE"
				availability_type:           "ZONAL"
				disk_size:                   10
				disk_type:                   "PD_SSD"
				disk_autoresize:             true
				deletion_protection_enabled: true
				backup_configuration: {
					enabled:                        true
					point_in_time_recovery_enabled: true
				}
				ip_configuration: {
					ipv4_enabled:       false
					private_network:    "projects/example/global/networks/dev"
					ssl_mode:           "ENCRYPTED_ONLY"
					allocated_ip_range: "google-services"
				}
			}
		}
		assert: input.out.resource.google_sql_database["dagster-dev-dagster"] == {
			name:     "dagster"
			instance: "${google_sql_database_instance.dagster-dev.name}"
		}
		assert: input.out.resource.google_sql_database["dagster-dev-analytics"].#import == "example/dagster-dev/analytics"
		assert: input.out.resource.google_sql_user["dagster-dev-dagster"] == {
			name:     "dagster"
			instance: "${google_sql_database_instance.dagster-dev.name}"
			password: "${random_password.dagster.result}"
		}
		assert: input.out.resource.google_sql_user["dagster-dev-readonly"].name == "readonly"

		assert: input.refs == {
			instance: "google_sql_database_instance.dagster-dev"
			databases: {
				dagster:   "google_sql_database.dagster-dev-dagster"
				analytics: "google_sql_database.dagster-dev-analytics"
			}
			users: {
				dagster:  "google_sql_user.dagster-dev-dagster"
				readonly: "google_sql_user.dagster-dev-readonly"
			}
		}
	}
}

cloudSqlResult: [for _, test in #CloudSqlTests {test.assert & true}]
