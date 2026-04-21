# Print an optspec for argparse to handle cmd's options that are independent of any subcommand.
function __fish_tm_global_optspecs
	string join \n v/version update-bin h/help
end

function __fish_tm_needs_command
	# Figure out if the current invocation already has a command.
	set -l cmd (commandline -opc)
	set -e cmd[1]
	argparse -s (__fish_tm_global_optspecs) -- $cmd 2>/dev/null
	or return
	if set -q argv[1]
		# Also print the command, so this can be used to figure out what it is.
		echo $argv[1]
		return 1
	end
	return 0
end

function __fish_tm_using_subcommand
	set -l cmd (__fish_tm_needs_command)
	test -z "$cmd"
	and return 1
	contains -- $cmd[1] $argv
end

complete -c tm -n "__fish_tm_needs_command" -s v -l version -d 'Print version'
complete -c tm -n "__fish_tm_needs_command" -l update-bin -d 'Update the program from the latest version in the repository'
complete -c tm -n "__fish_tm_needs_command" -s h -l help -d 'Print help'
complete -c tm -n "__fish_tm_needs_command" -f -a "list" -d 'List installed applications'
complete -c tm -n "__fish_tm_needs_command" -f -a "list-installed" -d 'List installed applications'
complete -c tm -n "__fish_tm_needs_command" -f -a "remove" -d 'Uninstall apps (Ex: tm remove discord waterfox)'
complete -c tm -n "__fish_tm_needs_command" -f -a "install" -d 'Install applications from specific tarballs or repositories'
complete -c tm -n "__fish_tm_needs_command" -f -a "update" -d 'Update installed applications from repositories'
complete -c tm -n "__fish_tm_needs_command" -f -a "repo" -d 'Manage repositories'
complete -c tm -n "__fish_tm_needs_command" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c tm -n "__fish_tm_using_subcommand list" -s h -l help -d 'Print help'
complete -c tm -n "__fish_tm_using_subcommand list-installed" -s h -l help -d 'Print help'
complete -c tm -n "__fish_tm_using_subcommand remove" -s h -l help -d 'Print help'
complete -c tm -n "__fish_tm_using_subcommand install" -s a -l app-name -d 'Custom name for the application (single install only)' -r
complete -c tm -n "__fish_tm_using_subcommand install" -s u -l use-root -d 'Whether to use root/pkexec (single install only)' -r
complete -c tm -n "__fish_tm_using_subcommand install" -s c -l category -d 'Category for the application (single install only)' -r
complete -c tm -n "__fish_tm_using_subcommand install" -s h -l help -d 'Print help'
complete -c tm -n "__fish_tm_using_subcommand update" -s h -l help -d 'Print help'
complete -c tm -n "__fish_tm_using_subcommand repo; and not __fish_seen_subcommand_from list pkg-list pkg-search sync add remove help" -s h -l help -d 'Print help'
complete -c tm -n "__fish_tm_using_subcommand repo; and not __fish_seen_subcommand_from list pkg-list pkg-search sync add remove help" -f -a "list" -d 'List all repositories and their package counts'
complete -c tm -n "__fish_tm_using_subcommand repo; and not __fish_seen_subcommand_from list pkg-list pkg-search sync add remove help" -f -a "pkg-list" -d 'List all packages available in the repositories'
complete -c tm -n "__fish_tm_using_subcommand repo; and not __fish_seen_subcommand_from list pkg-list pkg-search sync add remove help" -f -a "pkg-search" -d 'Search for packages in the repositories'
complete -c tm -n "__fish_tm_using_subcommand repo; and not __fish_seen_subcommand_from list pkg-list pkg-search sync add remove help" -f -a "sync" -d 'Fetch latest default and community repositories from GitHub'
complete -c tm -n "__fish_tm_using_subcommand repo; and not __fish_seen_subcommand_from list pkg-list pkg-search sync add remove help" -f -a "add" -d 'Add a third-party repository'
complete -c tm -n "__fish_tm_using_subcommand repo; and not __fish_seen_subcommand_from list pkg-list pkg-search sync add remove help" -f -a "remove" -d 'Remove a third-party repository'
complete -c tm -n "__fish_tm_using_subcommand repo; and not __fish_seen_subcommand_from list pkg-list pkg-search sync add remove help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c tm -n "__fish_tm_using_subcommand repo; and __fish_seen_subcommand_from list" -s h -l help -d 'Print help'
complete -c tm -n "__fish_tm_using_subcommand repo; and __fish_seen_subcommand_from pkg-list" -s h -l help -d 'Print help'
complete -c tm -n "__fish_tm_using_subcommand repo; and __fish_seen_subcommand_from pkg-search" -s h -l help -d 'Print help'
complete -c tm -n "__fish_tm_using_subcommand repo; and __fish_seen_subcommand_from sync" -s h -l help -d 'Print help'
complete -c tm -n "__fish_tm_using_subcommand repo; and __fish_seen_subcommand_from add" -l requires-root
complete -c tm -n "__fish_tm_using_subcommand repo; and __fish_seen_subcommand_from add" -s h -l help -d 'Print help'
complete -c tm -n "__fish_tm_using_subcommand repo; and __fish_seen_subcommand_from remove" -s h -l help -d 'Print help'
complete -c tm -n "__fish_tm_using_subcommand repo; and __fish_seen_subcommand_from help" -f -a "list" -d 'List all repositories and their package counts'
complete -c tm -n "__fish_tm_using_subcommand repo; and __fish_seen_subcommand_from help" -f -a "pkg-list" -d 'List all packages available in the repositories'
complete -c tm -n "__fish_tm_using_subcommand repo; and __fish_seen_subcommand_from help" -f -a "pkg-search" -d 'Search for packages in the repositories'
complete -c tm -n "__fish_tm_using_subcommand repo; and __fish_seen_subcommand_from help" -f -a "sync" -d 'Fetch latest default and community repositories from GitHub'
complete -c tm -n "__fish_tm_using_subcommand repo; and __fish_seen_subcommand_from help" -f -a "add" -d 'Add a third-party repository'
complete -c tm -n "__fish_tm_using_subcommand repo; and __fish_seen_subcommand_from help" -f -a "remove" -d 'Remove a third-party repository'
complete -c tm -n "__fish_tm_using_subcommand repo; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c tm -n "__fish_tm_using_subcommand help; and not __fish_seen_subcommand_from list remove install update repo help" -f -a "list" -d 'List installed applications'
complete -c tm -n "__fish_tm_using_subcommand help; and not __fish_seen_subcommand_from list remove install update repo help" -f -a "remove" -d 'Uninstall apps (Ex: tm remove discord waterfox)'
complete -c tm -n "__fish_tm_using_subcommand help; and not __fish_seen_subcommand_from list remove install update repo help" -f -a "install" -d 'Install applications from specific tarballs or repositories'
complete -c tm -n "__fish_tm_using_subcommand help; and not __fish_seen_subcommand_from list remove install update repo help" -f -a "update" -d 'Update installed applications from repositories'
complete -c tm -n "__fish_tm_using_subcommand help; and not __fish_seen_subcommand_from list remove install update repo help" -f -a "repo" -d 'Manage repositories'
complete -c tm -n "__fish_tm_using_subcommand help; and not __fish_seen_subcommand_from list remove install update repo help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c tm -n "__fish_tm_using_subcommand help; and __fish_seen_subcommand_from repo" -f -a "list" -d 'List all repositories and their package counts'
complete -c tm -n "__fish_tm_using_subcommand help; and __fish_seen_subcommand_from repo" -f -a "pkg-list" -d 'List all packages available in the repositories'
complete -c tm -n "__fish_tm_using_subcommand help; and __fish_seen_subcommand_from repo" -f -a "pkg-search" -d 'Search for packages in the repositories'
complete -c tm -n "__fish_tm_using_subcommand help; and __fish_seen_subcommand_from repo" -f -a "sync" -d 'Fetch latest default and community repositories from GitHub'
complete -c tm -n "__fish_tm_using_subcommand help; and __fish_seen_subcommand_from repo" -f -a "add" -d 'Add a third-party repository'
complete -c tm -n "__fish_tm_using_subcommand help; and __fish_seen_subcommand_from repo" -f -a "remove" -d 'Remove a third-party repository'
