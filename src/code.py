from adafruit_macropad import MacroPad
import time

macropad = MacroPad()

text_lines = macropad.display_text(title="behold!")
key_commands = ["Toggle Discord Mute", "", "Change Light Mode", "", "", "", "", "", "", "", "", ""]
num_modes = 4

mode = 1
last_position = 0

while True:
    key_event = macropad.keys.events.get()
    
    text_lines[0].text=" "
    text_lines[1].text=" "

    if key_event:
        if key_event.pressed:
            
            text_lines[1].text=key_commands[key_event.key_number]
            macropad.pixels[key_event.key_number] = (52, 199, 220)

            # Discord Mute
            if key_event.key_number == 0:
                macropad.keyboard.press(
                    macropad.Keycode.CAPS_LOCK
                )
                macropad.keyboard.release_all()

            # Change Light Mode
            if key_event.key_number == 2:
                mode = mode+1 if mode < 4 else 1
                if mode == 1:
                    macropad.pixels.brightness = 0.5
                if mode == 2:
                    macropad.pixels.brightness = 0.25
                if mode == 3:
                    macropad.pixels.brightness = 0.05
                if mode == 4:
                    macropad.pixels.brightness = 0.0
            # TODO RGB MODES and stuff

        time.sleep(0.4)

    macropad.encoder_switch_debounced.update()

    if macropad.encoder_switch_debounced.pressed:
        macropad.consumer_control.send(
                macropad.ConsumerControlCode.PLAY_PAUSE
        )

    current_position = macropad.encoder

    if macropad.encoder > last_position:
        macropad.consumer_control.send(
                macropad.ConsumerControlCode.VOLUME_INCREMENT
        )
        last_position = current_position

    if macropad.encoder < last_position:
        macropad.consumer_control.send(
                macropad.ConsumerControlCode.VOLUME_DECREMENT
        )
        last_position = current_position
    
    macropad.pixels.fill((219, 123, 48))

    text_lines.show()