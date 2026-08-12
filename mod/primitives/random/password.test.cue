@if(test)

package random

#PasswordTests: {
	"defaults": {
		input: #Password & {in: name: "database"}

		assert: input.out.resource.random_password.database == {
			length:  32
			lower:   true
			upper:   true
			numeric: true
			special: true
		}
	}

	"rotation-keeper": {
		input: #Password & {in: {
			name: "database"
			keepers: revision: "2"
		}}

		assert: input.out.resource.random_password.database.keepers.revision == "2"
	}
}

passwordResult: [for _, test in #PasswordTests {test.assert & true}]
