# Sessionman
This is an implementation of a session manager. It will provide similar features to systemd-logind but without the dependency on systemd as an init system.
Currently it is more or less in concept stage. The text below refers to features that should exist in the future but do not yet. 

## Why would I want this
Session management seems to be very important to many people when talking about the features systemd brought to linux desktop. What a session manager actually does is 
relatively simple as a concept: watch which users session is currently active and allow that user access to hardware like monitors and sound-devices.

Theoretically there could me multiple sets of monitors + mouse + keyboard. Those are called "seat"s and obviously every one of those can have a session concurretly with the others.

For this to work new sessions have to be (de)registered with sessionman. This can happen with the pam_exec module or your session startup mus contain 
a call to the registration tool.

Sessionman watches over these interactions and grants the needed access rights by using the acl api (see "man acl").

The rest of this readme is some more or less structured random info.

## Things sessionman needs to do
Track active vt with /sys/class/tty0/active. This file is inotify watchable. This is somewhat hacky but afaik there is no better way to watch for this.

Device <-> Seat matching is done by (e)udev (I think. It might just throw everything into seat0. But honestly this is for PCs not terminal servers anyways...)
If devices dont show up in that matching they are considered global/shared devices

User <-> Session matching is done by sessionman (get from pam module?)
Session <-> tty matching is done by sessionman (get from pam module?)
tty <-> Seat matching is done by sessionman (how does that work? Can the pam module check which seat it is runing on somehow? Can we check to which seat a tty belomgs somehow?)


## Session registering tool
Gets called by pam_exec or from a normal script when the session starts

1. Gather info about new logins and send these to sessionman
    1. PID by getppid()
    1. uid by getuid() (is set correctly by pam_exec)
1. Receive a session token from sessionman and store in $XDG_SESSION_ID and maybe set other $XDG_SESSION_* env vars
    1. Write those env vars out on stdout for a session script to set in it's env vars
    1. Write those env vars in a file for pam_env to read in. This poses a race condition but I cant think of a better way
        that does not involve writing my own pam module.


## Session deregistering tool
Gets called by pam_exec or from a normal script when the session exits

1. Send sessionman the session-id in $XDG_SESSION_ID that is ending

## Sessionman
Needs to be started before any sessions are.

1. Track active vt. If it changes set access rights for the devices in the seat so the user in the active session can access them.
1. Create new session on a login event for that user and move given pid into a new cgroup
1. Tear down sessions on a logout event. Dont kill processes in the cgroup. Keep watching the cgroup.events file and delete the cgroup if its empty

## Moving to cgroups
When is this done exactly? If done in sessionman there could be a race between:
1. The sessions first process forking and exiting instantly to start the sessions other processes
2. sessionman moving the session into a cgroup

systemd-logind does it in the pam module (see "man pam_systemd"). This moves "login", "sddm-helper", etc... processes into the cgroup as well. Apparently that is a thing 
display managers (or their helper processes if they spawn any) have to be able to deal with (but really, they shouldn't care in which cgroup they are so it's probably fine).  

### Possible solution
sessionman could require a session to register itself with it. Afterall this is not a security feature but a nicety feature. And it should be relatively trivial
to make a write to an unix socket and wait for an answer. This could even be wrapped in a small shell script distributed with sessionman so its literally one call of a shell script.

This way we avoid:
1. Races
2. Weird side effects in a PAM module (moving stuff into cgroups seems to be a weird sideeffect to me atleast)
3. Having to create PAM modules in the first place. We can use the existing pam_exec and pam_env to do this for us.

## Random info
Just some helpful links on the topic

https://dvdhrm.wordpress.com/2013/08/24/how-vt-switching-works/
https://dvdhrm.wordpress.com/2013/08/24/session-management-on-linux/
https://www.freedesktop.org/wiki/Software/systemd/multiseat/
