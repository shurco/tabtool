package app.shurco.tabtool

/** Result encoding shared by every bridge call: `OK`, `OK:<value>`, `CANCELLED` or `ERROR:<message>`. */
internal const val OK = "OK"
internal const val CANCELLED = "CANCELLED"
private const val ERROR = "ERROR:"

internal fun ok(value: String? = null): String =
    if (value.isNullOrEmpty()) OK else "$OK:$value"

internal fun failure(message: String): String =
    "$ERROR${message.replace('\n', ' ').replace('\r', ' ').take(300)}"
