# twitch_likes

## Usage

    Just use The Provided Executable from the release section.
    Then Place Your Channel id in a file called channel.txt wich is generated at first execution.
    once setup and running you just need to add a browser source in obs and set the URL:

    http://localhost:35395/

    commands are:
    !like
    !dislike
    !refundlike
    !lurk

    if you dont want the lurks counter just add this in the custom css field to your obs browser source:

    #lurks { display: none; }

## Building from source

    Just Clone The repo and then Cargo Build/run it.
