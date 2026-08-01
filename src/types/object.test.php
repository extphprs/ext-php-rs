<?php

class StringableOk
{
    public function __toString(): string
    {
        return 'hello';
    }
}

class NotStringableAtAll
{
    public int $x = 1;
}
